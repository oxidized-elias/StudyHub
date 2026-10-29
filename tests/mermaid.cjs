// Exercise the browser diagram lifecycle without loading React or a CDN runtime.
const fs=require('fs'),vm=require('vm'),assert=require('node:assert/strict');
const html=fs.readFileSync('web/index.html','utf8');
const source=html.slice(html.indexOf('function Md('),html.indexOf(' const click=e=>',html.indexOf('function Md(')));
const calls=[],nodes=[],effects=[];
const element=()=>({className:'',textContent:'',isConnected:true,children:[],append(...items){this.children.push(...items)}});
const codes=['flowchart LR\nA --> B','sequenceDiagram\nAlice->>Bob: Hello','mindmap\n  root((Notes))\n    Topic','invalid diagram'].map(text=>{
 const code={className:'language-mermaid',textContent:text};
 code.parentElement={replaceWith(node){nodes.push(node)}};return code;
});
const root={children:[],querySelectorAll(selector){return selector==='pre > code'?codes:[]}};
const context={React:{useRef:()=>({current:root}),useMemo:fn=>fn()},useEffect:fn=>effects.push(fn),render:md=>md,
 document:{createElement:element,getElementById:()=>null},DOMPurify:{sanitize:svg=>svg},
 mermaid:{async render(id,text){calls.push({id,text});if(text==='invalid diagram')throw Error('Invalid syntax');return {svg:'<svg></svg>',bindFunctions(node){node.bound=true}}}},
 diagramQueue:Promise.resolve(),diagramId:0};
vm.createContext(context);
vm.runInContext(source+'}\nMd({md:"example"});',context);
const cleanup=effects[0]();
(async()=>{
 await context.diagramQueue;
 assert.equal(calls.length,4);
 assert.equal(new Set(calls.map(call=>call.id)).size,4);
 for(const node of nodes.slice(0,3)){assert.equal(node.innerHTML,'<svg></svg>');assert.equal(node.bound,true)}
 assert.equal(nodes[3].className,'diagram-error');
 assert.match(nodes[3].children[1].children[1].textContent,/invalid diagram\n\nInvalid syntax/);
 cleanup();
 console.log('PASS: Mermaid diagram dispatch, unique IDs, SVG insertion, bindings and invalid-source fallback.');
})().catch(error=>{console.error(error);process.exitCode=1});
