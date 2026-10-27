const assert=require('node:assert/strict');
const {harness,nodes}=require('./request_audit.cjs');
const close=tree=>nodes(tree?.props?.action,n=>n.type==='IconButton'&&n.props['aria-label']==='Dismiss Warning')[0];
function warnings(){
 const h=harness(),saved=new Map();h.c.localStorage.getItem=k=>saved.get(k)||null;h.c.localStorage.setItem=(k,v)=>saved.set(k,v);
 const action={type:'Button',props:{children:'View Jobs'}};
 const props={severity:'warning',dismissKey:'generation-job-4',action,children:'Generation failed'};
 let tree=h.render(h.e.Notice,props);assert(close(tree));assert(nodes(tree.props.action,n=>n===action).length,'preserve existing action');
 close(tree).props.onClick({});assert.equal(h.render(h.e.Notice,props),null);
 // Remount (e.g. after refresh) still hides this unchanged job failure.
 h.values[0]=null;assert.equal(h.render(h.e.Notice,props),null);
 assert(close(h.render(h.e.Notice,{...props,children:'A new failure'})),'changed failure should be visible');
 assert(close(h.render(h.e.Notice,{...props,dismissKey:'generation-job-5'})),'new jobs should remain visible');
 const error=h.render(h.e.Notice,{severity:'error',children:'Connection failed'});assert(close(error));
 let cleared=false;const existing=h.render(h.e.Notice,{severity:'success',children:'Done',action,onClose:()=>cleared=true});close(existing).props.onClick({});assert.equal(cleared,true);
 assert.equal(close(h.render(h.e.Notice,{severity:'info',children:'Instructions'})),undefined,'leave ordinary instructions unchanged');
}
warnings();console.log('PASS: warning/error close buttons, retained actions, persistent job dismissal, changed-error visibility, and existing close callbacks.');
for(const file of ['web/index.html','apps/SelfHosted-AI_Tests/web/index.html','apps/StudyHub/web/index.html']){
 const source=require('fs').readFileSync(file,'utf8');
 assert.match(source,/borderRadius:'8px',alignItems:'center',py:1,minHeight:48/);
 assert.match(source,/'& \.MuiAlert-message':\{minWidth:0,py:0/);
 assert.match(source,/'& \.MuiAlert-action':\{py:0,alignItems:'center',alignSelf:'center'\}/);
}
