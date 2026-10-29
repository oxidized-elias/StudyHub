const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
for(const file of ['web/index.html','apps/SelfHosted-AI_Tests/web/index.html']){
 const html=fs.readFileSync(file,'utf8'),ctx={};vm.createContext(ctx);
 vm.runInContext(html.slice(html.indexOf('const escapeHtml='),html.indexOf('const normalizeMath=')),ctx);
 const graph={title:'Parabola <script>',x:[-5,5],y:[-5,5],series:[{type:'line',points:[[-2,4],[0,0,'origin'],[2,4]]},{type:'points',points:[[1,-2,'A']]}]};
 const render=g=>ctx.coordinateGraph(JSON.stringify(g));
 const svg=render(graph);assert.match(svg,/<polyline points="204,108 300,300 396,108"/);assert.match(svg,/cx="348" cy="396"/);assert.match(svg,/origin/);assert.match(svg,/&lt;script&gt;/);assert.doesNotMatch(svg,/<script>/);
 assert.throws(()=>render({...graph,x:[1,1]}));assert.throws(()=>render({...graph,series:[{type:'line',points:[[100,0]]}]}));
 assert.throws(()=>render({...graph,series:[{type:'line',points:[['0',0]]}]}));assert.throws(()=>render({...graph,series:[{type:'function',points:[[0,0]]}]}));
 assert.throws(()=>ctx.coordinateGraph('not JSON'));assert.throws(()=>ctx.coordinateGraph(' '.repeat(100001)));
 const branches=render({...graph,series:[{type:'line',points:[[-2,-1],[-1,-2]]},{type:'line',points:[[1,2],[2,1]]}]});assert.equal((branches.match(/<polyline/g)||[]).length,2);
 assert.match(html,/language-graph/);assert.match(html,/USE_PROFILES:\{svg:true\}/);
}
console.log('PASS: coordinate graph projection, labels, separate branches, escaped text, invalid data and bounded input in both viewers');
