const assert=require('node:assert/strict');
const {harness,nodes,label}=require('./request_audit.cjs');
const button=(tree,text)=>nodes(tree,n=>n.type==='Button'&&label(n)===text)[0];
async function confirmation(){
 const h=harness();let refreshed=0;
 const props={kind:'journal',id:17,title:'Journal Entry',details:'Remove the journal note only.',data:{question_id:3},onDeleted:async()=>refreshed++};
 let tree=h.render(h.e.DeleteHistoryButton,props);
 assert.equal(nodes(tree,n=>n.type==='Dialog')[0].props.open,false);
 button(tree,'Delete').props.onClick();tree=h.render(h.e.DeleteHistoryButton,props);
 assert.equal(nodes(tree,n=>n.type==='Dialog')[0].props.open,true);
 assert.equal(h.requests.length,0,'opening confirmation must not delete');
 button(tree,'Cancel').props.onClick();assert.equal(h.values[2],false);assert.equal(h.requests.length,0);
 tree=h.render(h.e.DeleteHistoryButton,props);button(tree,'Delete').props.onClick();
 tree=h.render(h.e.DeleteHistoryButton,props);await button(tree,'Delete Permanently').props.onClick();
 assert.equal(h.requests[0].url,'/api/history/journal/17');assert.equal(h.requests[0].options.method,'DELETE');
 assert.equal(JSON.parse(h.requests[0].options.body).question_id,3);assert.equal(refreshed,1);assert.equal(h.values[2],false);
 const disabled=h.render(h.e.DeleteHistoryButton,{...props,disabled:true});assert.equal(button(disabled,'Delete').props.disabled,true);
}
function categories(){
 const props={history:{artifacts:[{id:1,title:'Quiz',thread_id:2}],attempts:[{id:3,title:'Quiz',artifact_id:1}]},threads:[{id:2,title:'Chat'}],jobs:[{id:4,status:'running',request:{kind:'quiz'}}],progress:{},notebooks:[],navigate(){},reload:async()=>{}};
 for(const [tab,kind] of [[0,'material'],[1,'session'],[2,'conversation'],[3,'progress'],[4,'job']]){
  const h=harness([false,'',tab,0]),tree=h.render(h.e.History,props);
  const del=nodes(tree,n=>n.type===h.e.DeleteHistoryButton&&n.props.kind===kind)[0];
  assert(del,`missing ${kind} deletion`);
  if(kind==='job')assert.equal(del.props.disabled,true,'running jobs finish before deletion');
 }
 const h=harness([false,'',[{attempt_id:8,question_id:2,title:'Problem',concept:'Algebra'}]]);
 const tree=h.render(h.e.ProblemJournal,{navigate(){}});
 const del=nodes(tree,n=>n.type===h.e.DeleteHistoryButton)[0];assert.equal(del.props.id,8);assert.equal(del.props.data.question_id,2);
}
(async()=>{categories();await confirmation();console.log('PASS: History deletion controls, scoped journal IDs, confirmation/cancel, DELETE request, refresh, and running-job protection.');})().catch(e=>{console.error(e);process.exit(1)});
