const fs=require('fs'),vm=require('vm');
const ts=require(process.env.TYPESCRIPT_RUNTIME||'typescript');
const html=fs.readFileSync('web/index.html','utf8');
const source=html.split('<script type="text/babel">')[1].split('</script>')[0].replace("ReactDOM.createRoot(document.getElementById('root')).render(<App/>);",'globalThis.ViewerComponent=Viewer;globalThis.SourcePreviewComponent=SourcePreview;globalThis.UploadComponent=Upload;globalThis.ReviewComponent=Review;globalThis.HumanReviewComponent=HumanReview;globalThis.MdComponent=Md;globalThis.waitDiagrams=()=>diagramQueue;');
const compiled=ts.transpileModule(source,{fileName:'index.jsx',compilerOptions:{jsx:ts.JsxEmit.React,target:ts.ScriptTarget.ES2020},reportDiagnostics:true});
if(compiled.diagnostics.length)throw Error('JSX diagnostics');
async function test({failFirst=false,transportError=false,skipHuman=false,course='',subject=''}={}){
 let states=[true,null,[],[{name:'a.md'},{name:'b.pdf'},{name:'c.png'}],[],false,null,null,false,{course_name:course,class_type:subject},null,skipHuman],cursor=0,submissions=[],polls=[];
 const React={useState:init=>{const id=cursor++;return [states[id]===undefined?init:states[id],v=>states[id]=typeof v==='function'?v(states[id]):v]},useRef:()=>({current:false}),useEffect:()=>{},createElement:(type,props,...children)=>({type,props:props||{},children})};
 const context={window:{__APP_BASE__:'',__EMBEDDED__:false},React,MaterialUI:new Proxy({},{get:(_,k)=>k}),mermaid:{initialize(){}},FormData:class{constructor(){this.fields=[]}append(k,v){this.fields.push([k,v])}},setTimeout:fn=>{fn()},fetch:async(url,opts)=>{
  if(url==='/api/submit'){
   submissions.push(opts.body.fields);
   if(transportError)throw Error('offline');
   return {ok:true,json:async()=>({session_id:submissions.length})};
  }
  const id=Number(url.split('/').pop());polls.push(id);
  return {ok:true,json:async()=>({id,status:failFirst&&id===1?'failed':'done',title:'Note '+id,error:failFirst&&id===1?'bad input':''})};
 }};
 vm.createContext(context);vm.runInContext(compiled.outputText,context);
 const tree=context.UploadComponent({ver:1,refresh:async()=>{},sessions:[]});
 const find=node=>{if(!node||typeof node!=='object')return null;if(node.type==='Button'&&node.props.onClick&&node.children.some(x=>typeof x==='string'&&x.startsWith('Process ')))return node;for(const child of node.children||[]){const found=Array.isArray(child)?child.map(find).find(Boolean):find(child);if(found)return found}return null};
 const button=find(tree);if(!button)throw Error('No bulk button');await button.props.onClick();
 if(transportError){if(submissions.length!==1||states[3].length!==3)throw Error('Transport failure lost queued files');}
 else{if(submissions.length!==3||states[3].length!==0||states[2].length!==3)throw Error('Queue did not finish');if(failFirst&&states[2][0].ok!==false)throw Error('Failed file not reported');}
 for(const request of submissions){
  if((request.find(([k])=>k==='course_name')?.[1]||'')!==course)throw Error('Bulk course was not sent');
  if((request.find(([k])=>k==='class_type')?.[1]||'')!==subject)throw Error('Bulk class was not sent');
 }
 for(const request of submissions){if(request.some(([k,v])=>k==='skip_human'&&v==='true')!==skipHuman)throw Error('Bulk skip-human preference was not sent correctly');}
 for(const request of submissions){if(request.filter(([k])=>k==='file').length!==1||!request.some(([k,v])=>k==='unknown'&&v==='true'))throw Error('Wrong multipart request');}
}

function nodes(tree,predicate){if(!tree||typeof tree!=='object')return [];return [...(predicate(tree)?[tree]:[]),...(tree.children||[]).flatMap(c=>Array.isArray(c)?c.flatMap(x=>nodes(x,predicate)):nodes(c,predicate))]}
async function noteLog(selected){
 const note={id:3,title:'Polynomials',lesson:'Polynomials',class_type:'Math',course:'PreCalculus',status:'done',events:[{kind:'ai1',data:{}}],note:'Notes'};
 let states=[[],selected?note:null,2,'',false,'',''],cursor=0,requests=[];
 const React={useState:init=>{const id=cursor++;return [states[id]===undefined?init:states[id],v=>states[id]=typeof v==='function'?v(states[id]):v]},useEffect:()=>{},createElement:(type,props,...children)=>({type,props:props||{},children})};
 const context={window:{__APP_BASE__:'',__EMBEDDED__:false},React,MaterialUI:new Proxy({},{get:(_,k)=>k}),mermaid:{initialize(){}},fetch:async(url,opts)=>{requests.push([url,opts]);return {ok:true,json:async()=>({cleared:1})}}};vm.createContext(context);vm.runInContext(compiled.outputText,context);
 const tree=context.ReviewComponent({sessions:[],refresh:async()=>{},sel:selected?3:null,setSel:()=>{},onDelete:()=>{}});
 const buttons=nodes(tree,n=>n.type==='Button'&&n.children.includes('Clear AI Log'));
 if(!selected){if(buttons.length)throw Error('Clear AI Log appeared without a selected note');return}
 if(buttons.length!==1)throw Error('Missing note-specific Clear AI Log');
 if(buttons[0].props.sx.bgcolor!=='#8b272c'||buttons[0].props.variant!=='contained')throw Error('Log clear button is not dark red');
 await buttons[0].props.onClick();
 if(requests[0][0]!=='/api/review/session/3/logs'||requests[0][1].method!=='DELETE')throw Error('Log clearing targeted more than one note');
}
async function humanApproval(){
 const draft={title:'Polynomials',body:'AI wording',summary:'Degree',unit_name:'Polynomials',feedback:''};
 const edits={...draft,body:'My corrected wording',feedback:'Use my definition.'};
 const q={draft,edits,sources:[],transcript:'Original',locked_title:false,class_type:'Math',course:'PreCalculus'};
 let states=[q,edits,0,false,'',''],cursor=0,requests=[];
 const React={useState:init=>{const id=cursor++;return [states[id]===undefined?init:states[id],v=>states[id]=typeof v==='function'?v(states[id]):v]},useEffect:()=>{},createElement:(type,props,...children)=>({type,props:props||{},children})};
 const context={window:{__APP_BASE__:'',__EMBEDDED__:false},React,MaterialUI:new Proxy({},{get:(_,k)=>k}),mermaid:{initialize(){}},fetch:async(url,opts)=>{requests.push([url,opts]);return {ok:true,json:async()=>({ok:true})}}};vm.createContext(context);vm.runInContext(compiled.outputText,context);
 const tree=context.HumanReviewComponent({id:3,refresh:async()=>{}});
 const button=nodes(tree,n=>n.type==='Button'&&n.children.includes('Save reviewed note'))[0];if(!button||button.props.disabled)throw Error('Human approval button is missing or disabled');
 await button.props.onClick();
 const body=JSON.parse(requests[0][1].body);
 if(body.body!==edits.body||body.feedback!==edits.feedback||body.action!=='approve')throw Error('Human corrections were not submitted exactly');
}
function savedSourceViewer(){
 let states=[],cursor=0;
 const React={useState:init=>{const id=cursor++;return [states[id]===undefined?init:states[id],v=>states[id]=v]},useEffect:()=>{},createElement:(type,props,...children)=>({type,props:props||{},children})};
 const context={window:{__APP_BASE__:'',__EMBEDDED__:false},React,MaterialUI:new Proxy({},{get:(_,k)=>k}),mermaid:{initialize(){}}};
 vm.createContext(context);vm.runInContext(compiled.outputText,context);
 const sources=[{url:'/original.pdf',type:'application/pdf',name:'Original.pdf'}];
 let tree=context.SourcePreviewComponent({sources,transcript:'Saved transcript'});
 if(!nodes(tree,n=>n.props.component==='iframe'&&n.props.src==='/original.pdf').length)throw Error('Original upload is missing');
 const toggle=nodes(tree,n=>n.type==='ToggleButtonGroup')[0];
 toggle.props.onChange(null,'transcript');cursor=0;
 tree=context.SourcePreviewComponent({sources,transcript:'Saved transcript'});
 if(!nodes(tree,n=>n.props.component==='pre'&&n.children.includes('Saved transcript')).length)throw Error('Transcription toggle failed');
 if(nodes(tree,n=>n.props.component==='iframe').length)throw Error('Original remained visible in transcription mode');
 if(!html.includes('component="details"')||!html.includes('View Source · Transcription or Original Upload'))throw Error('Collapsible saved source viewer is missing');
}
(async()=>{savedSourceViewer();await test();await test({skipHuman:true});await test({course:'Precalculus',subject:'Math'});await test({subject:'Physics'});await test({failFirst:true});await test({transportError:true});await noteLog(false);await noteLog(true);await humanApproval();console.log('PASS: JSX, bulk queue success/failure/recovery, note-only dark red log button, and exact human-review submission.');})().catch(e=>{console.error(e);process.exit(1)});
