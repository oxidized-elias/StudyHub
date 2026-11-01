const fs=require('fs'),vm=require('vm'),assert=require('node:assert/strict');
for(const [file,route] of [['web/index.html','/notes/api/review/session/42/pdf/source-diagram-42-1-0.png'],['apps/SelfHosted-AI_Tests/web/index.html','/learn/api/notes/42/pdf/source-diagram-42-1-0.png']]){
 const html=fs.readFileSync(file,'utf8');const start=html.indexOf("  el.querySelectorAll('img').forEach(img=>{");const end=html.indexOf("  el.querySelectorAll('pre > code')",start);
 const image={getAttribute:()=> '../Uploaded%20Notes/source-diagram-42-1-0.png',style:{}};
 const unrelated={getAttribute:()=> 'example.png',style:{}};
 const context={el:{querySelectorAll:()=>[image,unrelated]},appUrl:p=>(file.startsWith('web/')?'/notes':'/learn')+p};
 vm.createContext(context);vm.runInContext(html.slice(start,end),context);
 assert.equal(image.src,route);assert.equal(image.style.maxWidth,'100%');assert.equal(unrelated.src,undefined);
}
assert.match(fs.readFileSync('web/index.html','utf8'),/label="Use original diagram images"/);
console.log('PASS: original diagram image routing in both viewers and settings toggle.');
