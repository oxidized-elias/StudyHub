const assert=require('node:assert/strict');
const fs=require('node:fs');
const {harness,nodes,label}=require('./request_audit.cjs');
function select(){
 const profile={id:1,kind:'ollama',base_url:'http://remote:11434',model:'saved:4b'};
 const props={profile,profiles:[profile,{kind:'ollama',base_url:profile.base_url,model:'other:9b'},{kind:'ollama',base_url:'http://different',model:'wrong-server'}]};
 let chosen;props.onChange=v=>chosen=v;
 const h=harness([['installed:9b'],false,'',false,0]);
 let tree=h.render(h.e.ModelSelect,props),field=nodes(tree,n=>n.type==='TextField'&&n.props.label==='Model')[0];
 assert.equal(field.props.select,true);assert.equal(field.props.value,'saved:4b');
 const options=nodes(tree,n=>n.type==='MenuItem').map(n=>n.props.value);
 assert(options.includes('installed:9b'));assert(options.includes('other:9b'));assert(!options.includes('wrong-server'));
 field.props.onChange({target:{value:'installed:9b'}});assert.equal(chosen,'installed:9b');
 field.props.onChange({target:{value:'__custom__'}});tree=h.render(h.e.ModelSelect,props);
 const custom=nodes(tree,n=>n.type==='TextField'&&n.props.label==='Custom Model ID')[0];assert(custom);custom.props.onChange({target:{value:'private-model'}});assert.equal(chosen,'private-model');
 const cloud=h.render(h.e.ModelSelect,{...props,profile:{...profile,kind:'claude'}});assert(!nodes(cloud,n=>n.type==='Button'&&label(n)==='Refresh Models').length);
 const root=fs.readFileSync('web/index.html','utf8');assert(root.includes('SettingsModelSelect key={s.provider+\'-author\'}'));assert(root.includes('SettingsModelSelect key={s.provider+\'-vision\'}'));
}
async function discovery(){
 const h=harness();h.c.setTimeout=fn=>{h.pending=fn;return 1};h.c.clearTimeout=()=>{};
 const profile={id:1,kind:'ollama',base_url:'http://remote:11434',model:'saved',api_key:''};
 h.render(h.e.ModelSelect,{profile,profiles:[],onChange(){}});h.effects[0]();
 h.c.fetch=async(url,options)=>{assert.equal(url,'/api/providers/models');assert.equal(JSON.parse(options.body).base_url,profile.base_url);return{ok:true,json:async()=>({models:['server-model']})}};
 await h.pending();assert.equal(h.values[0][0],'server-model');assert.equal(h.values[1],false);
}
(async()=>{select();await discovery();console.log('PASS: Settings dropdowns, remote-server discovery, saved model preservation, endpoint filtering, and custom model fallback.');})().catch(e=>{console.error(e);process.exit(1)});
