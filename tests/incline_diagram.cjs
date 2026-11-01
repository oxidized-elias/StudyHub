const fs=require('fs'),vm=require('vm'),assert=require('node:assert/strict');
for(const file of ['web/index.html','apps/SelfHosted-AI_Tests/web/index.html']){
 const html=fs.readFileSync(file,'utf8');const start=html.indexOf('function inclineDiagram('),end=html.indexOf('\nfunction ',start+10);
 const ctx={};vm.createContext(ctx);vm.runInContext(html.slice(start,end),ctx);
 const render=d=>ctx.inclineDiagram(JSON.stringify(d));
 const input={type:'inclined_plane',angle:30,label:'Toolbox <script>',forces:['gravity','normal','friction_up']};
 const svg=render(input);
 assert.match(svg,/rotate\(-30/);assert.match(svg,/Toolbox &lt;script&gt;/);
 const lines=[...svg.matchAll(/<line x1="([^"]+)" y1="([^"]+)" x2="([^"]+)" y2="([^"]+)"/g)].map(m=>m.slice(1).map(Number));
 assert.equal(lines.length,3);
 const vectors=lines.map(([x,y,ex,ey])=>[ex-x,ey-y]);
 assert.equal(vectors[0][0],0);assert(vectors[0][1]>0,'gravity down');
 assert(vectors[1][0]<0&&vectors[1][1]<0,'normal up and away from ramp');
 assert(vectors[2][0]>0&&vectors[2][1]<0,'static friction up ramp');
 assert(Math.abs(vectors[1][0]*vectors[2][0]+vectors[1][1]*vectors[2][1])<1e-8,'normal perpendicular to slope');
 assert.equal((render({...input,forces:[]}).match(/<line /g)||[]).length,0,'do not invent forces');
 assert.throws(()=>render({...input,angle:90}));assert.throws(()=>render({...input,forces:['invented']}));
}
console.log('PASS: incline geometry, force directions, perpendicular normal, escaped labels and invalid input in both viewers.');
