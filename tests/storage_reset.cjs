const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync('apps/StudyHub/src/main.rs','utf8');
const template=source.match(/r#"<head><script>(.*?)<\/script>"#/s)[1];
const code=template.replace('const epoch={};','const epoch="new-install";').replaceAll('{{','{').replaceAll('}}','}');
function storage(values={}){
 const store={...values};
 Object.defineProperties(store,{
  getItem:{value:k=>store[k]??null},setItem:{value:(k,v)=>{store[k]=v}},removeItem:{value:k=>delete store[k]}
 });
 return store;
}
const local=storage({'hub.notifications':'old','study.notebook':'41','dismissed-warning.job-7':'old','other.app':'keep'});
const session=storage({'study.job':'7','other.session':'keep'});
vm.runInNewContext(code,{localStorage:local,sessionStorage:session});
assert.equal(local['hub.notifications'],undefined);assert.equal(local['study.notebook'],undefined);
assert.equal(local['dismissed-warning.job-7'],undefined);assert.equal(session['study.job'],undefined);
assert.equal(local['other.app'],'keep');assert.equal(session['other.session'],'keep');
assert.equal(local['studyhub.data-epoch'],'new-install');
local['study.notebook']='2';vm.runInNewContext(code,{localStorage:local,sessionStorage:session});
assert.equal(local['study.notebook'],'2');
vm.runInNewContext(code,{localStorage:{getItem(){throw Error('blocked')}}});
console.log('PASS: new data clears stale app browser state once, preserves unrelated keys and tolerates blocked storage.');
