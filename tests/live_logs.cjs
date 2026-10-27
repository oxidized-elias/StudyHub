const assert=require('node:assert/strict');
const {harness,nodes,label}=require('./request_audit.cjs');
const turn=()=>new Promise(resolve=>setImmediate(resolve));
async function logs(){
 const h=harness([true,[],'',null,false,Date.now()]);h.c.URLSearchParams=URLSearchParams;
 const timers=new Map();let timerId=0;h.c.setTimeout=fn=>{timers.set(++timerId,fn);return timerId};h.c.clearTimeout=id=>timers.delete(id);
 const calls=[];let polls=0;
 h.c.fetch=async url=>{calls.push(url);if(url.includes('/diagnostics'))return{ok:true,json:async()=>({model:'Omni-Edu-9B',installed:true,loaded:false,context_configured:16384,message:'Not loaded'})};polls++;return{ok:true,json:async()=>({entries:polls===1?[{id:10,created:1,stage:'request_started',message:'Preparing',detail:{provider_id:2}},{id:11,created:1,stage:'provider_wait',message:'Waiting',detail:{}}]:[{id:12,created:2,stage:'request_failed',message:'Not enough memory',detail:{}}]})}};
 let tree=h.render(h.e.LiveLogs,{jobId:9,providerId:1});const stop=h.effects[1]();await turn();
 assert(calls[0].includes('job_id=9'));assert(calls[0].includes('after=0'));assert.equal(h.values[1].length,2);
 tree=h.render(h.e.LiveLogs,{jobId:9,providerId:1});
 assert(nodes(tree,n=>n.type==='Box'&&n.props.role==='log').length);
 await nodes(tree,n=>n.type==='Button'&&label(n)==='Check Model Status')[0].props.onClick();
 assert.equal(calls[1],'/api/providers/2/diagnostics','use logged provider, not a newly selected profile');assert.equal(h.values[3].loaded,false);
 const next=[...timers.values()][0];timers.clear();await next();
 assert(calls[2].includes('after=11'));assert.equal(h.values[1].length,3);
 stop();assert.equal(timers.size,0,'stop polling when the panel unmounts');
 const closed=harness([false,[],'',null,false,Date.now()]);closed.c.URLSearchParams=URLSearchParams;
 closed.render(closed.e.LiveLogs,{jobId:9});closed.effects[1]();assert.equal(closed.requests.length,0,'closed panels must not poll');
}
logs().then(()=>console.log('PASS: live scoped log polling, incremental cursor, provider diagnostics, retained errors, and polling cleanup.')).catch(e=>{console.error(e);process.exit(1)});
