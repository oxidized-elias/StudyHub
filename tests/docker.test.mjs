import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { checkHealth } from '../scripts/docker-healthcheck.mjs';

const root=fileURLToPath(new URL('..',import.meta.url));
const healthy={service:'studyhub',ok:true,notes:true,learning:true};
test('container health check authenticates without putting credentials in its URL',async()=>{
 let call;
 assert.equal(await checkHealth({STUDYHUB_USER:'owner',STUDYHUB_PASSWORD:'test:password'},async(url,options)=>{
  call={url,options};return{ok:true,json:async()=>healthy};
 }),true);
 assert.equal(call.url,'http://127.0.0.1:8082/api/health');
 assert.equal(call.options.headers.Authorization,`Basic ${Buffer.from('owner:test:password').toString('base64')}`);
 assert.equal(call.options.redirect,'error');
});
test('health check rejects auth errors, incomplete startup and network failures',async()=>{
 assert.equal(await checkHealth({},async(_url,options)=>{assert.deepEqual(options.headers,{});return{ok:false}}),false);
 for(const body of [{...healthy,notes:false},{...healthy,learning:false},{...healthy,service:'unrelated'},{}]){
  assert.equal(await checkHealth({},async()=>({ok:true,json:async()=>body})),false);
 }
 await assert.rejects(()=>checkHealth({},async()=>{throw Error('unavailable')}));
});
const dockerAvailable=!spawnSync('docker',['compose','version'],{encoding:'utf8'}).error;
const renderedConfigs=process.env.STUDYHUB_COMPOSE_DEFAULT_CONFIG&&process.env.STUDYHUB_COMPOSE_CUSTOM_CONFIG;
function configuration(overrides={}){
 const rendered=overrides.STUDYHUB_VAULT_PATH?process.env.STUDYHUB_COMPOSE_CUSTOM_CONFIG:process.env.STUDYHUB_COMPOSE_DEFAULT_CONFIG;
 if(rendered)return JSON.parse(readFileSync(rendered,'utf8')).services.studyhub;
 const env={...process.env};
 for(const key of ['STUDYHUB_HOST','STUDYHUB_PORT','STUDYHUB_URL','STUDYHUB_USER','STUDYHUB_PASSWORD','STUDYHUB_VAULT_PATH','STUDYHUB_UID','STUDYHUB_GID'])delete env[key];
 Object.assign(env,overrides);
 const result=spawnSync('docker',['compose','--env-file','/dev/null','-f','compose.yaml','config','--format','json'],{cwd:root,env,encoding:'utf8'});
 assert.equal(result.status,0,result.stderr);
 return JSON.parse(result.stdout).services.studyhub;
}
test('Compose keeps container paths stable and persists both data and the vault',{skip:!dockerAvailable&&!renderedConfigs},()=>{
 const config=configuration({STUDYHUB_BIND:'127.0.0.1:9999',DATA_DIR:'/host/data',VAULT_DIR:'/host/vault'});
 assert.equal(config.environment.STUDYHUB_BIND,'0.0.0.0:8082');
 assert.equal(config.environment.DATA_DIR,'/app/data');
 assert.equal(config.environment.LEARNING_DATA_DIR,'/app/data/learning');
 assert.equal(config.environment.VAULT_DIR,'/vault');
 assert.equal(config.ports[0].host_ip,'127.0.0.1');
 assert.equal(config.ports[0].published,'8082');
 assert.equal(config.environment.STUDYHUB_URL,'http://localhost:8082');
 assert.equal(config.init,true);
 for(const target of ['/app/data','/vault'])assert.ok(config.volumes.some(v=>v.target===target&&v.type==='volume'));
});
test('Compose supports host vaults, custom ports and authentication',{skip:!dockerAvailable&&!renderedConfigs},()=>{
 const config=configuration({STUDYHUB_VAULT_PATH:'/tmp/studyhub-host-vault',STUDYHUB_PORT:'8182',STUDYHUB_PASSWORD:'test-password'});
 assert.equal(config.environment.STUDYHUB_URL,'http://localhost:8182');
 assert.equal(config.ports[0].published,'8182');
 assert.equal(config.environment.STUDYHUB_PASSWORD,'test-password');
 assert.ok(config.volumes.some(v=>v.type==='bind'&&v.source==='/tmp/studyhub-host-vault'&&v.target==='/vault'));
});
test('image packages one server, logo, health script and file extraction tools',()=>{
 const dockerfile=readFileSync(new URL('../Dockerfile',import.meta.url),'utf8');
 assert.match(dockerfile,/COPY logo\.png/);
 assert.match(dockerfile,/poppler-utils imagemagick libheif-examples/);
 assert.match(dockerfile,/USER node/);
 assert.match(dockerfile,/"--release", "--no-build"/);
 assert.match(dockerfile,/docker-healthcheck\.mjs/);
});
