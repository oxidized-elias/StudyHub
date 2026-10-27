// Real browser DOM checks without a network server or external React dependency.
const fs=require('node:fs'),os=require('node:os'),path=require('node:path'),assert=require('node:assert/strict'),{spawnSync}=require('node:child_process');
const ts=require(process.env.TYPESCRIPT_RUNTIME||'typescript');
const html=fs.readFileSync('apps/SelfHosted-AI_Tests/web/index.html','utf8');
const source=html.slice(html.indexOf('const readableMathMarkdown='),html.indexOf('const mathHtml='))+html.slice(html.indexOf('const powerPattern='),html.indexOf('function KillTaskButton('));
const js=ts.transpileModule(source,{compilerOptions:{jsx:ts.JsxEmit.React,target:ts.ScriptTarget.ES2020}}).outputText;
const setup=`const effects=[];const useEffect=f=>effects.push(f);const React={forwardRef:f=>f,useRef:x=>({current:x}),createElement:(tag,props)=>{const el=document.createElement(tag);for(const [k,v]of Object.entries(props)){if(k==='ref')v(el);else if(k==='style')Object.assign(el.style,v);else if(k.startsWith('on'))el.addEventListener(k.slice(2).toLowerCase(),v);else if(k==='contentEditable')el.contentEditable=String(v);else if(!['suppressContentEditableWarning','className'].includes(k)&&v!=null)el.setAttribute(k,String(v));}return el;}};const escapeHtml=v=>String(v).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));const mathHtml=t=>'<span>'+escapeHtml(t)+'</span>';`;
const checks=`
const equal=(a,b)=>{if(a!==b)throw Error(JSON.stringify(a)+' != '+JSON.stringify(b))};
let value='',sent=0;
const root=RichMarkdownControl({value,onChange:e=>value=e.target.value,onKeyDown:e=>{if(e.key==='Enter'&&!e.shiftKey){e.preventDefault();sent++;}}},null);document.body.append(root);effects.forEach(f=>f());root.focus();restoreEditorCaret(root,0);
function type(text){for(const ch of text){const s=window.getSelection(),r=s.getRangeAt(0);r.deleteContents();const node=document.createTextNode(ch);r.insertNode(node);r.setStartAfter(node);r.collapse(true);s.removeAllRanges();s.addRange(r);root.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertText',data:ch}));}}
const key=(key,props={})=>root.dispatchEvent(new KeyboardEvent('keydown',{key,bubbles:true,cancelable:true,...props}));
type('2^3');equal(value,'2^3');equal(root.querySelector('sup').textContent,'3');equal(editorSource(root),value);
type('4');equal(value,'2^34');equal(root.querySelector('sup').textContent,'34');key('Backspace');equal(value,'2^3');
key('Enter',{shiftKey:true});type('**bold**');equal(value,'2^3\\n**bold**');equal(root.querySelector('strong').textContent,'bold');equal(editorSource(root),value);
key('Enter');equal(sent,1);equal(value,'2^3\\n**bold**');key('z',{ctrlKey:true});equal(value,'2^3\\n**bold*');key('z',{ctrlKey:true,shiftKey:true});equal(value,'2^3\\n**bold**');
restoreEditorCaret(root,0);type('A ');equal(value,'A 2^3\\n**bold**');
restoreEditorCaret(root,value.length);const clipboard=new DataTransfer();clipboard.setData('text/plain','\\n<img src=x onerror=alert(1)>');root.dispatchEvent(new ClipboardEvent('paste',{clipboardData:clipboard,bubbles:true,cancelable:true}));equal(value,'A 2^3\\n**bold**\\n<img src=x onerror=alert(1)>');equal(root.querySelectorAll('img').length,0);
const range=document.createRange();range.selectNodeContents(root);const selection=window.getSelection();selection.removeAllRanges();selection.addRange(range);const copied=new DataTransfer();root.dispatchEvent(new ClipboardEvent('copy',{clipboardData:copied,bubbles:true,cancelable:true}));equal(copied.getData('text/plain'),value);
document.body.setAttribute('data-result','PASS');
`;
const dir=fs.mkdtempSync(path.join(os.tmpdir(),'notes-rich-input-'));const file=path.join(dir,'test.html');
fs.writeFileSync(file,'<!doctype html><body><script>'+setup+js+'try{'+checks+'}catch(e){document.body.setAttribute("data-result","FAIL");document.body.setAttribute("data-error",e.message)}</script>');
const result=spawnSync('/usr/bin/chromium',['--headless','--no-sandbox','--disable-dev-shm-usage','--disable-gpu','--disable-background-networking','--no-first-run','--user-data-dir='+path.join(dir,'profile'),'--dump-dom','file://'+file],{encoding:'utf8',timeout:15000});
try{assert.equal(result.status,0,result.stderr||String(result.error));assert.match(result.stdout,/<body data-result="PASS"/,result.stdout.match(/data-error="[^"]*/)?.[0]||result.stderr);console.log('PASS: Chromium rich input typing, powers, caret restoration, multiline Markdown, Enter, undo/redo, paste safety and raw-source copy.');}finally{fs.rmSync(dir,{recursive:true,force:true});}
