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

async function savedProfileSettings(){
 const profiles=[{id:1,label:'First',kind:'ollama',base_url:'http://remote:11434',model:'old',context:8192,api_key_set:true},{id:2,label:'Saved Gemma',kind:'ollama',base_url:'http://remote:11434',model:'gemma4:e4b',context:16384}];
 const settings={unified:true,active_provider:'2'};
 const h=harness();
 let reloaded=0;
 const props={profiles,settings,reload:async()=>{reloaded++},onTutorial(){}};
 let tree=h.render(h.e.ProviderSettings,props);
 const select=nodes(tree,n=>n.type===h.e.ModelSelect)[0];
 assert.equal(select.props.profile.id,2);assert.equal(select.props.profile.model,'gemma4:e4b');
 h.c.fetch=async(url,options)=>{
  h.requests.push({url,options});
  if(url==='/api/providers')return{ok:true,json:async()=>({...JSON.parse(options.body),api_key:undefined})};
  return{ok:true,json:async()=>({})};
 };
 select.props.onChange('new-installed-model');
 tree=h.render(h.e.ProviderSettings,props);
 const button=nodes(tree,n=>n.type==='Button'&&label(n)==='Save & Use Model')[0];assert(button);
 await button.props.onClick();
 const payload=JSON.parse(h.requests.find(r=>r.url==='/api/providers').options.body);
 assert.equal(payload.id,2);assert.equal(payload.model,'new-installed-model');assert.equal(reloaded,1);
 tree=h.render(h.e.ProviderSettings,props);
 assert.equal(nodes(tree,n=>n.type===h.e.ModelSelect)[0].props.profile.id,2);
 const initial=h.e.initialModelProfile(profiles,settings);
 assert.equal(initial.id,2);assert.equal(initial.api_key,'');
 assert.equal(h.e.selectedProfileId(profiles,settings,1),2);
 assert.equal(h.e.selectedProfileId(profiles,{active_provider:'999'},1),1);
 const remounted=harness();
 tree=remounted.render(remounted.e.ProviderSettings,{...props,profiles:[profiles[0],{...profiles[1],model:payload.model}]});
 assert.equal(nodes(tree,n=>n.type===remounted.e.ModelSelect)[0].props.profile.model,'new-installed-model');
}
(async()=>{select();await discovery();await savedProfileSettings();console.log('PASS: Settings dropdowns, remote-server discovery, saved model preservation, endpoint filtering, and custom model fallback.');})().catch(e=>{console.error(e);process.exit(1)});
