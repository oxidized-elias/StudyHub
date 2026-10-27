"""Offline API regression test with a fake Ollama; uses only temporary data.
Run after cargo build: python tests/smoke.py
"""
import json, os, pathlib, re, socket, subprocess, tempfile, threading, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.request import Request, urlopen

calls = []
class Mock(BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        prompt = body['messages'][0]['content']
        user = body['messages'][1]['content']
        existing = user.split('EXISTING NOTES:\n', 1)[-1].split('\n\nTRANSCRIPT:', 1)[0] if 'EXISTING NOTES:' in user else ''
        match = re.search(r'\[\[([^\]]+)\]\]', existing)
        target = match[1] if match else None
        calls.append(prompt)
        assert 'moderator' not in prompt.lower()
        if prompt.startswith('Infer metadata'):
            answer = dict(class_type='Physics', course='Mechanics', lesson='Momentum', unit_name='Motion and Forces')
        elif prompt.startswith('You are planning'):
            answer = dict(big_ideas=[dict(name='Momentum', evidence='Momentum is mass times velocity', connects_to=target or 'SELF: Impulse', how='Impulse changes momentum'),dict(name='Impulse', evidence='Impulse changes momentum', connects_to='SELF: Momentum', how='Impulse changes momentum')], questions=[])
        else:
            answer = dict(title='Momentum', unit_name='Motion and Forces', summary='Momentum and impulse.', body='## Momentum\nMomentum is mass times velocity. Impulse changes momentum.\n```mermaid\nflowchart LR\n Impulse --> Momentum\n```', links=[])
        if 'body' in answer and target:
            answer['title'] = 'Impulse'
            answer['body'] += f'\nImpulse connects to momentum in [[{target}]].'
            answer['links'] = [dict(target=target, how='Impulse changes momentum')]
        raw = json.dumps({'message': {'content': json.dumps(answer)}}).encode()
        self.send_response(200); self.end_headers(); self.wfile.write(raw)
    def log_message(self, *args): pass

def port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0)); return sock.getsockname()[1]

with tempfile.TemporaryDirectory(prefix='notes-smoke-') as tmp:
    root = pathlib.Path(tmp); data = root/'data'; data.mkdir()
    mock = ThreadingHTTPServer(('127.0.0.1', 0), Mock)
    threading.Thread(target=mock.serve_forever, daemon=True).start()
    (data/'config.json').write_text(json.dumps(dict(ollama_url=f'http://127.0.0.1:{mock.server_port}', vault_path=str(root/'vault'), web_search=False, lessons_use=False, models={'ollama':dict(ai1='author',ai2='legacy',vision='vision')})))
    origin = f'http://127.0.0.1:{port()}'
    base = origin + '/notes'
    proc = subprocess.Popen(['target/debug/study-hub'], env={**os.environ, 'DATA_DIR':str(data),'STUDYHUB_BIND':origin.removeprefix('http://'), 'LEARNING_DATA_DIR':str(root/'learning')}, stdout=subprocess.DEVNULL)
    def api(path, body=None, content='application/json', method=None):
        with urlopen(Request(base+path, data=body, headers={'Content-Type':content}, method=method), timeout=10) as response:
            return json.load(response)
    try:
        for _ in range(100):
            try: api('/api/settings'); break
            except OSError: time.sleep(.05)
        settings=api('/api/settings'); assert 'ai2' not in settings['models']['ollama']
        for unknown in (False, True):
            fields={'unknown':'true'} if unknown else {'class_type':'Physics','course_name':'Mechanics','lesson':'Impulse'}
            boundary='smoke-boundary'
            chunks=[f'--{boundary}\r\nContent-Disposition: form-data; name="{k}"\r\n\r\n{v}\r\n' for k,v in fields.items()]
            chunks.append(f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="notes.md"\r\nContent-Type: text/plain\r\n\r\nMomentum is mass times velocity. Impulse changes momentum.\r\n--{boundary}--\r\n')
            sid=api('/api/submit',''.join(chunks).encode(),f'multipart/form-data; boundary={boundary}')['session_id']
            for _ in range(100):
                row=api(f'/api/review/session/{sid}')
                if row['status'] not in ('running','waiting'): break
                time.sleep(.05)
            assert row['status']=='review',row
            assert not row['note'], 'An unapproved upload must not write a final note'
            pending=api(f'/api/review/session/{sid}/human')
            assert pending['sources'], 'The original file must be available for comparison'
            edits={**pending['edits'], 'body':pending['edits']['body']+'\n\nMy reviewed wording.', 'feedback':'Keep my wording for definitions.'}
            api(f'/api/review/session/{sid}/human',json.dumps(edits).encode(),method='PUT')
            assert api(f'/api/review/session/{sid}/human')['edits']['body']==edits['body']
            api(f'/api/review/session/{sid}/human',json.dumps({**edits,'action':'approve'}).encode())
            row=api(f'/api/review/session/{sid}')
            assert row['status']=='done' and 'My reviewed wording.' in row['note'],row
            lessons=api('/api/lessons')
            assert lessons[0]['kind']=='user_edit' and '+My reviewed wording.' in lessons[0]['changes']
            assert (data/'lessons'/f"{lessons[0]['id']}.md").exists()
            assert row['unit']=='' and row['class_type']=='Physics' and row['course']=='Mechanics',row
            assert 'unit_name: "Motion and Forces"' in row['note'],row
            assert not any(e['kind'] in ('ai2','debate') for e in row['events'])
            assert all((root/'vault'/'Physics'/'Mechanics'/'Uploaded Notes'/name).exists() for name in row['pdfs'])
            # Verify optional unit and inferred unit name also survive regeneration.
            api(f'/api/review/session/{sid}/redo',b'{"guidance":""}')
            pending=api(f'/api/review/session/{sid}/human')
            assert api(f'/api/review/session/{sid}')['status']=='review'
            # Regeneration leaves the previous note intact until the human approves.
            assert 'My reviewed wording.' in api(f'/api/review/session/{sid}')['note']
            api(f'/api/review/session/{sid}/human',json.dumps({**pending['edits'],'action':'approve'}).encode())
            assert 'unit_name: "Motion and Forces"' in api(f'/api/review/session/{sid}')['note']
        print(f'PASS: known and unknown uploads, optional units, source relocation, human review and lessons, regeneration, legacy config; {len(calls)} author calls, no moderator.')
    finally:
        proc.terminate(); proc.wait(timeout=5); mock.shutdown()
