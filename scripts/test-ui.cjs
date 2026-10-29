// Run every socket-free UI regression. Runtimes can be supplied through environment variables.
const {spawnSync}=require('node:child_process');
const {resolve}=require('node:path');
const tests=['coordinate_graph','mermaid','ui','suite_ui','request_audit','experience','study_workflows','keyboard','history_delete','model_select','warning_dismiss','live_logs','task_controls','notifications','storage_reset','subject_ideas','render','learning_render'];
for(const test of tests){
  const result=spawnSync(process.execPath,[`tests/${test}.cjs`],{cwd:resolve(__dirname,'..'),stdio:'inherit',env:process.env});
  if(result.error){console.error(result.error.message);process.exit(1)}
  if(result.status!==0)process.exit(result.status||1);
}
