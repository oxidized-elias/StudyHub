// Requires a running Docker daemon and a built image. Uses only disposable test volumes.
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import assert from 'node:assert/strict';

const image=process.argv[2]||'studyhub:smoke';
const name=`studyhub-smoke-${randomUUID().slice(0,8)}`;
const volumes=[`${name}-data`,`${name}-vault`];
const password=randomUUID();
const authorization=`Basic ${Buffer.from(`smoke:${password}`).toString('base64')}`;
function docker(args,{allowFailure=false}={}){
 const result=spawnSync('docker',args,{encoding:'utf8',timeout:120000});
 if(!allowFailure&&(result.error||result.status!==0))throw Error(result.error?.message||result.stderr.trim()||'Docker command failed');
 return result.stdout?.trim()||'';
}
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function start(){
 docker(['run','-d','--init','--name',name,'-p','127.0.0.1::8082',
  '-e','STUDYHUB_USER=smoke','-e',`STUDYHUB_PASSWORD=${password}`,
  '-v',`${volumes[0]}:/app/data`,'-v',`${volumes[1]}:/vault`,image]);
 const port=docker(['inspect','--format','{{(index (index .NetworkSettings.Ports "8082/tcp") 0).HostPort}}',name]);
 const base=`http://127.0.0.1:${port}`;
 for(let attempt=0;attempt<90;attempt++){
  const health=docker(['inspect','--format','{{.State.Health.Status}}',name]);
  if(health==='healthy')return base;
  if(docker(['inspect','--format','{{.State.Running}}',name])!=='true')throw Error('Container exited during startup');
  await sleep(1000);
 }
 throw Error('Container never became healthy');
}
async function request(base,path,options={}){
 return fetch(base+path,{...options,headers:{Authorization:authorization,'Content-Type':'application/json',...options.headers},signal:AbortSignal.timeout(10000)});
}
try{
 for(const volume of volumes)docker(['volume','create',volume]);
 let base=await start();
 assert.equal((await fetch(base+'/api/health')).status,401);
 assert.equal((await request(base,'/api/health')).status,200);
 const logo=await request(base,'/logo.png');assert.equal(logo.status,200);assert.equal(logo.headers.get('content-type'),'image/png');
 assert.equal(Buffer.from(await logo.arrayBuffer()).subarray(0,8).toString('hex'),'89504e470d0a1a0a');
 assert.notEqual(docker(['exec',name,'id','-u']),'0');
 docker(['exec',name,'sh','-c','command -v pdftotext && command -v pdftoppm && command -v convert && command -v heif-convert']);
 const created=await request(base,'/learn/api/notebooks',{method:'POST',body:JSON.stringify({title:'Docker persistence check',goal:'Test storage',instructions:''})});
 assert.equal(created.status,200);const notebook=await created.json();
 docker(['exec',name,'node','-e',"require('node:fs').writeFileSync('/vault/docker-smoke.txt','persistent vault content')"]);
 docker(['stop','--time','15',name]);
 assert.equal(docker(['inspect','--format','{{.State.ExitCode}}',name]),'0');
 docker(['rm',name]);base=await start();
 const restored=await request(base,`/learn/api/notebooks/${notebook.id}`);
 assert.equal(restored.status,200);assert.equal((await restored.json()).title,'Docker persistence check');
 assert.equal(docker(['exec',name,'node','-e',"process.stdout.write(require('node:fs').readFileSync('/vault/docker-smoke.txt','utf8'))"]),'persistent vault content');
 console.log('PASS: container health/authentication, bundled logo, non-root runtime, extraction tools, graceful stop, and data/vault persistence across recreation.');
}catch(error){
 console.error(error.message);
 console.error(docker(['logs','--tail','80',name],{allowFailure:true}));
 process.exitCode=1;
}finally{
 docker(['rm','-f',name],{allowFailure:true});
 for(const volume of volumes)docker(['volume','rm',volume],{allowFailure:true});
}
