"""End-to-end suite check with a fake Ollama, temporary databases, and no paid API calls.
Requires local socket permission. Run after: cargo build --workspace --offline
"""
import json, os, pathlib, re, signal, socket, subprocess, tempfile, threading, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.request import Request, urlopen

ROOT = pathlib.Path(__file__).resolve().parents[1]
class Model(BaseHTTPRequestHandler):
    def do_POST(self):
        req=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        system=req['messages'][0]['content']; user=req['messages'][-1]['content']
        if system.startswith('Infer metadata'):
            out=dict(class_type='Math',course='PreCalculus',lesson='Polynomials',unit_name='Polynomials')
        elif system.startswith('You are planning'):
            out=dict(big_ideas=[dict(name='Degree',evidence='Highest exponent',connects_to='SELF: Roots',how='Degree limits roots'),dict(name='Roots',evidence='Zeros',connects_to='SELF: Degree',how='Roots relate to degree')],questions=[])
        elif system.startswith("You convert a student's"):
            out=dict(title='Polynomials',summary='Degree and roots',unit_name='Polynomials',body='## Degree\nDegree is the highest exponent and relates to roots.\n## Roots\nRoots are zeros and relate to degree.\n```mermaid\nflowchart LR\nDegree --> Roots\n```',links=[])
        elif system.startswith(('Create accurate study materials','Generate learning material')):
            kind=re.search(r'ACTIVITY (\w+)',user)[1]; count=int(re.search(r'Generate exactly (\d+) items',user)[1]); offset=(int(re.search(r'batch (\d+) of',user)[1])-1)*10; section=json.loads(re.search(r'SECTIONS: (.*)',user)[1])[0]
            if kind in ('quiz','practice_test'):
                out={'title':'Polynomial practice','questions':[dict(prompt=f'Degree question {offset+i+1}?',type='multiple_choice',choices=['One','Two'],correct_index=1,hint='Look at powers',explanation='Highest exponent.',section=section,concept="Degree",source_ids=[1],source_refs=[dict(source_id=1,section=1)]) for i in range(count)]}
            elif kind=='flashcards': out={'title':'Polynomial cards','cards':[dict(front=f'Degree {offset+i+1}?',back='Highest exponent',hint='Look at powers',section=section) for i in range(count)]}
            elif kind=='study_guide': out={'title':'Polynomial guide','body':'## Degree\nThe highest exponent.\n## Roots\nThe zeros.'}
            else: out={'title':'Explorer','html':'<button id="go">Explore</button>','css':'button{color:blue}','javascript':'Study.saveState({visited:true})'}
        elif system.startswith('Grade the learner'): out={'correct':True,'feedback':'Equivalent reasoning.'}
        elif system.startswith('Analyze study performance'): out='## Strengths\nYou identified degree.\n## Focus\nPractice the remaining questions.'
        elif 'polynomial explorer' in user and 'TOOL OBSERVATIONS:' not in user:
            out={'tool_calls':[{'name':'create_canvas','arguments':{'title':'Polynomial explorer','html':'<input type="range">','css':'','javascript':'Study.saveState({x:2})'}}]}
        elif 'TOOL OBSERVATIONS:' in user: out='Your interactive explorer is ready.'
        else: out='What do you already know about polynomial degree? [S1]'
        text=out if isinstance(out,str) else json.dumps(out)
        data=json.dumps({'message':{'content':text}}).encode(); self.send_response(200);self.end_headers();self.wfile.write(data)
    def log_message(self,*args): pass

def port():
    with socket.socket() as s: s.bind(('127.0.0.1',0));return s.getsockname()[1]

def wait(predicate):
    last=None
    for _ in range(300):
        try:
            last=predicate()
            if last:return last
        except OSError: pass
        time.sleep(.05)
    raise AssertionError(f'Timed out: {last}')

with tempfile.TemporaryDirectory(prefix='studyhub-smoke-') as tmp:
    root=pathlib.Path(tmp);hub_port=port()
    model=ThreadingHTTPServer(('127.0.0.1',0),Model);threading.Thread(target=model.serve_forever,daemon=True).start()
    base=f'http://127.0.0.1:{hub_port}'
    env={**os.environ,'DATA_DIR':str(root/'notes'),'VAULT_DIR':str(root/'vault'),'LEARNING_DATA_DIR':str(root/'learning'),'STUDYHUB_BIND':f'127.0.0.1:{hub_port}'}
    def api(path,data=None,method=None,content='application/json'):
        if data is not None and not isinstance(data,bytes):data=json.dumps(data).encode()
        with urlopen(Request(base+path,data=data,method=method,headers={'Content-Type':content}),timeout=20) as r:return json.load(r)
    log=open(root/'server.log','w+')
    hub=subprocess.Popen([str(ROOT/'target/debug/study-hub')],env=env,stdout=log,stderr=log)
    try:
        wait(lambda:api('/api/health').get('learning'))
        for prefix in ('/notes/','/learn/'):
            with urlopen(base+prefix) as r: assert f'window.__APP_BASE__="{prefix.rstrip("/")}"' in r.read().decode()
        s=api('/notes/api/settings');s.update(ollama_url=f'http://127.0.0.1:{model.server_port}',web_search=False,lessons_use=False);s['models']['ollama']['ai1']='fake';api('/notes/api/settings',s,'PUT')
        boundary='studyhub-smoke'
        fields={'course_name':'PreCalculus','class_type':'Math','lesson':'Polynomials'}
        chunks=[f'--{boundary}\r\nContent-Disposition: form-data; name="{k}"\r\n\r\n{v}\r\n' for k,v in fields.items()]
        chunks.append(f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="notes.md"\r\n\r\nDegree is the highest exponent. Roots are zeros.\r\n--{boundary}--\r\n')
        sid=api('/notes/api/submit',''.join(chunks).encode(),content=f'multipart/form-data; boundary={boundary}')['session_id']
        wait(lambda:api(f'/notes/api/review/session/{sid}')['status']=='review')
        review=api(f'/notes/api/review/session/{sid}/human');api(f'/notes/api/review/session/{sid}/human',{**review['edits'],'action':'approve'})
        assert api('/notes/api/integration/notes')[0]['id']==sid
        p=api('/learn/api/providers',dict(label='Fake local',kind='ollama',base_url=f'http://127.0.0.1:{model.server_port}',model='fake',context=32768))
        n=api('/learn/api/notebooks',dict(title='Math study',goal='Understand polynomials',instructions='One step at a time'))['id']
        api(f'/learn/api/notebooks/{n}/import',{'note_ids':[sid]})
        book=api(f'/learn/api/notebooks/{n}');assert 'Degree' in book['sources'][0]['body'];source=book['sources'][0]['id']
        def generate(kind,count=2,parent=None):
            j=api('/learn/api/generate',dict(kind=kind,prompt='Polynomials',notebook_id=n,provider_id=p['id'],count=count,parent_artifact_id=parent))['job_id']
            def complete():
                row=next(x for x in api('/learn/api/jobs') if x['id']==j)
                assert row['status']!='failed',row
                return row['artifact_id'] if row['status']=='done' else None
            return wait(complete)
        quiz=generate('quiz');attempt=api(f'/learn/api/artifacts/{quiz}/attempts',{})['id']
        assert 'correct_index' not in api(f'/learn/api/attempts/{attempt}')['artifact']['content']['questions'][0]
        assert api(f'/learn/api/attempts/{attempt}/answer',{'question_id':0,'response':1})['correct']
        result=api(f'/learn/api/attempts/{attempt}/finish',{})
        assert result['correct']==1 and result['skipped']==1 and result['percent']==50
        assert api(f'/learn/api/attempts/{attempt}/analyze',{})['analysis']
        flash=generate('flashcards',3);f=api(f'/learn/api/artifacts/{flash}/attempts',{})['id']
        api(f'/learn/api/attempts/{f}',{'order':[2,1,0],'position':1,'flipped':True},'PUT')
        assert api(f'/learn/api/attempts/{f}')['state']['position']==1
        guide=generate('study_guide');doc=api(f'/learn/api/artifacts/{guide}');revision=api(f'/learn/api/artifacts/{guide}',{'title':'Edited guide','content':{**doc['content'],'body':'## Corrected guide\nHuman wording'}},'PUT')['id'];assert revision!=guide
        token=api(f'/learn/api/artifacts/{quiz}/share',{})['token'];shared=api('/learn/api/shared/'+token);assert 'context' not in shared and 'thread_id' not in shared
        thread=api('/learn/api/threads',{'notebook_id':n,'provider_id':p['id'],'mode':'guided','title':'Learn polynomials'})['id']
        api(f'/learn/api/threads/{thread}/chat',{'message':'What is degree?'})
        tool=api(f'/learn/api/threads/{thread}/chat',{'message':'Build a polynomial explorer'})
        assert tool['artifact_ids'];assert api('/learn/api/artifacts/'+str(tool['artifact_ids'][0]))['kind']=='canvas'
        assert api('/learn/api/progress')['trend']
        assert len(api('/learn/api/history')['artifacts'])>=5
        print('PASS: unified server routing, note approval/import, selected sources, model adapter, quiz scoring, skips, analysis, flashcard resume, guide revisions, sharing, guided chat, UI-generation tools, and history.')
    except Exception:
        log.flush();log.seek(0);print(log.read()[-6000:]);raise
    finally:
        hub.send_signal(signal.SIGINT)
        try:hub.wait(timeout=5)
        except subprocess.TimeoutExpired:hub.kill();hub.wait()
        model.shutdown();log.close()
