import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile,mkdir,writeFile,chmod,symlink,rm} from 'node:fs/promises';
import {randomUUID,createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {scopeCheck,credentials,databaseCheck} from './live-work-preflight.mjs';
const fixture=JSON.parse(await readFile(new URL('../config/work-evaluation-fixtures.json',import.meta.url),'utf8')).cases[0];
const original=JSON.parse(await readFile(new URL('../config/live-work-evaluation-10a.json',import.meta.url),'utf8'));
test('frozen scope matches the existing Python execution digest',()=>{
 const checked=scopeCheck(original,fixture);assert.equal(checked.scope_valid,true);
 const report=JSON.parse(execFileSync('python3',['scripts/live-work-evaluation.py','--plan','config/live-work-evaluation-10a.json'],{encoding:'utf8'}));
 assert.equal(checked.scope_sha256,report.scope_sha256);assert.equal(checked.paid_requests,0);assert.equal(report.ready,Boolean(original.project_id));assert.equal(report.readiness_basis,'scope_structure_only');assert.equal(report.paid_execution_authorized,false);
});
test('tampered evidence, database targeting, nonce and limits fail closed',()=>{
 for(const mutate of [p=>p.preflight.task_input+=' ',p=>p.max_requests=2,p=>p.max_cost_microusd=4001,p=>p.model='other',p=>p.budget_id=p.agent_id,p=>p.preflight.database.name='primary',p=>p.preflight.database.socket_directory='/tmp/other',p=>p.preflight.request_id='bad']){
  const p=structuredClone(original);mutate(p);assert.equal(scopeCheck(p,fixture).scope_valid,false);
 }
});
test('credential diagnostics expose no key and reject public files and symlinks',async()=>{
 const path='/tmp/supa-m10a-secret-test-'+randomUUID();await mkdir(path,{mode:0o700});
 try{
  const key='sk-'+['not','a','real','credential'].join('-');await writeFile(path+'/private',`OPENAI_API_KEY=${key}\nOPENAI_PROJECT_ID=proj_fixture\n`,{mode:0o600});
  const c=await credentials({},path+'/private');assert.equal(c.report.key_basic_format_valid,true);assert.equal(c.report.project_id,'proj_fixture');assert.ok(!JSON.stringify(c.report).includes(key));
  await chmod(path+'/private',0o644);await assert.rejects(credentials({},path+'/private'),/unsafe_private_environment/);
  await chmod(path+'/private',0o600);await symlink(path+'/private',path+'/link');await assert.rejects(credentials({},path+'/link'),/unsafe_private_environment/);
 }finally{await rm(path,{recursive:true});}
});
test('read-only database identity, unpaid state and fresh independent budget are enforced',async()=>{
 const p=structuredClone(original),tag=randomUUID().replaceAll('-','').slice(0,12),base='/tmp/supa-m10a-pg-'+tag;
 await mkdir(base,{mode:0o700});p.preflight.database.data_directory=base+'/data';p.preflight.database.socket_directory=base+'/s';
 const files=(await import('node:fs/promises')).readdir;
 const migrations=[];for(const f of (await files('migrations')).filter(f=>/^\d+_.+\.sql$/.test(f))){migrations.push({version:Number(f.split('_')[0]),success:true,checksum:createHash('sha384').update(await readFile('migrations/'+f)).digest('hex')});}
 const sample={database:p.preflight.database.name,data_directory:base+'/data',listen_addresses:'',system_identifier:p.preflight.database.system_identifier,epochs:0,agents:1,grants:1,owner:p.preflight.test_owner_id,agent:{lifecycle:'paused',memory:[]},job:{agent_id:p.agent_id,customer_id:p.preflight.test_owner_id,revision:0,network:'simulated',state:'created',input_sha256:p.input_sha256,received_atoms:0,earned_atoms:0,expires_at:p.preflight.job_expires_at,not_expired:true},jobs:1,live_sessions:0,inference_calls:0,cycles:0,actions:0,schedules:0,economic_entries:0,budgets:0,budget:null,migrations};
 const execute=async(_command,args,options)=>{assert.match(args.at(-1),/^BEGIN READ ONLY;/);assert.match(options.env.PGOPTIONS,/default_transaction_read_only=on/);assert.equal(options.env.PGPASSFILE,'/dev/null');return {stdout:JSON.stringify(sample)}};
 try{
  assert.equal((await databaseCheck(p,{execute})).budget_state,'proposal_only');
  await assert.rejects(databaseCheck(p,{execute,forExecution:true}),/provisioning_requires_approval/);
  sample.budgets=1;sample.budget={id:p.budget_id,limit_microusd:5000,committed_microusd:0};assert.equal((await databaseCheck(p,{execute,forExecution:true})).budget_state,'installed_zero_spent');
  for(const mutate of [s=>s.system_identifier='other',s=>s.owner=randomUUID(),s=>s.job.earned_atoms=1,s=>s.job.not_expired=false,s=>s.live_sessions=1,s=>s.budget.committed_microusd=1,s=>s.migrations[0].checksum='bad']){
   const backup=structuredClone(sample);mutate(sample);await assert.rejects(databaseCheck(p,{execute}));for(const key of Object.keys(sample))delete sample[key];Object.assign(sample,backup);
  }
 }finally{await rm(base,{recursive:true});}
});
