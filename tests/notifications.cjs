const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const ts=require(process.env.TYPESCRIPT_RUNTIME||'typescript');
const {harness,nodes,label}=require('./request_audit.cjs');
const source=fs.readFileSync('apps/StudyHub/web/index.html','utf8').split('<script type="text/babel">')[1].split('</script>')[0];
const js=ts.transpileModule(source,{compilerOptions:{jsx:ts.JsxEmit.React,target:ts.ScriptTarget.ES2020}}).outputText.replace(/ReactDOM.createRoot\(document.getElementById\('root'\)\).render\(React.createElement\(App, null\)\);?/,'');
async function completion(){
 const h=harness(['home',false,null,false,true,[],false,true,null,false,null]);let nativeCalls=0,interval;
 const c={...h.c,URLSearchParams,location:{search:'',origin:'https://study.test'},document:{hidden:false,addEventListener(){},removeEventListener(){}},setInterval:fn=>{interval=fn;return 1},clearInterval(){},Notification:class {static permission='granted';constructor(){nativeCalls++;throw Error('Desktop notification unavailable')}},fetch:async url=>({ok:true,json:async()=>url.includes('/notes/')?[{id:3,status:'done',title:'Polynomial Functions'}]:[]})};
 // React can evaluate functional state updates later, outside the poll's try/catch.
 const deferred=[],original=h.c.React.useState;c.React={...h.c.React,useState:init=>{const [v,set]=original(init);return[v,next=>{if(typeof next==='function')deferred.push(()=>set(next));else set(next)}]}};
 vm.createContext(c);vm.runInContext(js+'\nglobalThis.test={App,storeNotificationState,loadNotifications};',c);
 let tree=h.render(c.test.App,{});const frameCount=nodes(tree,n=>n.props.component==='iframe').length;
 const stop=h.effects[0]();for(let i=0;i<15;i++)await Promise.resolve();
 assert.equal(nativeCalls,1,'desktop side effects run safely outside a React state updater');
 deferred.forEach(fn=>fn());assert.equal(h.values[5].length,1);assert.equal(h.values[10].title,'Note saved');
 tree=h.render(c.test.App,{});const popup=nodes(tree,n=>n.type==='Snackbar')[0];
 assert.equal(popup.props.open,true);assert.equal(popup.children[0].type,'div','Grow must receive a native DOM element that accepts its ref');
 assert.equal(nodes(tree,n=>n.props.component==='iframe').length,frameCount,'notification keeps both apps mounted');
 await interval();assert.equal(nativeCalls,1,'unchanged completion must not notify again');
 popup.props.onClose();tree=h.render(c.test.App,{});assert.equal(nodes(tree,n=>n.type==='Snackbar')[0].props.open,false);assert.equal(nodes(tree,n=>n.type==='Snackbar')[0].children[0].type,'div','DOM animation target remains mounted during exit');
 const inbox=nodes(tree,n=>n.type==='Button'&&label(n).startsWith('Notifications'))[0];assert(inbox);stop();
 c.localStorage.setItem=()=>{throw Error('Storage full')};assert.doesNotThrow(()=>c.test.storeNotificationState('hub.notifications',[]));
 c.localStorage.getItem=()=>'{"invalid":"not an array"}';assert.equal(c.test.loadNotifications().length,0);
 c.localStorage.getItem=()=>JSON.stringify([null,{id:'bad',title:{}},{id:'note:1:done',title:'Saved',detail:'Notes',target:'notes'}]);assert.equal(c.test.loadNotifications().length,1);
}
completion().then(()=>console.log('PASS: notification enter/exit DOM target, completion deduplication, blocked desktop notifications, preserved app frames, and unavailable/corrupt storage.')).catch(e=>{console.error(e);process.exit(1)});
