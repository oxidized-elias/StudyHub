// Check that spatial SVG drawings render in both note viewers without Mermaid layout.
const fs=require('fs'),vm=require('vm'),assert=require('node:assert/strict');
for(const file of ['web/index.html','apps/SelfHosted-AI_Tests/web/index.html']){
 const html=fs.readFileSync(file,'utf8');
 const start=html.indexOf("   if(code.className.toLowerCase().split(/\\s+/).includes('language-svg')){");
 const end=html.indexOf("   if(code.className.toLowerCase().split(/\\s+/).includes('language-graph')){",start);
 assert(start>=0&&end>start);
 let replaced,warning,options;
 const svg={style:{}};
 const code={className:'language-svg',textContent:'<svg viewBox="0 0 300 200"><path d="M20 180 L280 30 L280 180 Z"/></svg>',parentElement:{replaceWith(node){replaced=node},before(node){warning=node}}};
 const context={code,document:{createElement:()=>({querySelector:()=>svg})},DOMPurify:{sanitize(text,config){options=config;return text}}};
 vm.createContext(context);
 vm.runInContext('(function(){'+html.slice(start,end)+'})()',context);
 assert.equal(replaced.className,'source-diagram');
 assert.match(replaced.innerHTML,/<path/);
 assert.equal(svg.style.height,'auto');
 assert(options.FORBID_TAGS.includes('foreignObject'));
 assert(options.FORBID_TAGS.includes('image'));
 assert(options.FORBID_ATTR.includes('href'));
 assert.equal(warning,undefined);
 replaced=undefined;context.document.createElement=()=>({querySelector:()=>null});
 vm.runInContext('(function(){'+html.slice(start,end)+'})()',context);
 assert.equal(replaced,undefined);
 assert.match(warning.textContent,/invalid SVG/);
}
console.log('PASS: spatial SVG dispatch, responsive sizing, sanitization policy and invalid SVG fallback in both viewers.');
