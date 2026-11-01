#!/usr/bin/env node
/** Read-only M10A checks. The only optional external operation is one model metadata GET. */
import {readFile,lstat,readdir} from 'node:fs/promises';
import {parseEnv} from 'node:util';
import {createHash} from 'node:crypto';
import {userInfo} from 'node:os';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import https from 'node:https';
const root=resolve(import.meta.dirname,'..'),exec=promisify(execFile);
const uuid=/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const archived='ad91e475-f1da-495a-8247-139643d2d8b1';
const hash=value=>createHash('sha256').update(value).digest('hex');
function canonical(value){if(Array.isArray(value))return value.map(canonical);if(value&&typeof value==='object')return Object.fromEntries(Object.keys(value).sort().map(k=>[k,canonical(value[k])]));return value;}
export function scopeCheck(plan,fixture){
 const p=plan.preflight,db=p?.database,blockers=[];
 if(!p||p.version!==1||!db||!/^supa_m10a_[0-9a-f]{12}_test$/.test(db.name??'')||!/^\/tmp\/supa-m10a-pg-[0-9a-f]{12}\/data$/.test(db.data_directory??'')||db.socket_directory!==db.data_directory.replace(/data$/,'s')||db.port!==55681||!/^\d{16,22}$/.test(db.system_identifier??''))blockers.push('unreviewed_disposable_database');
 const ids=[plan.agent_id,plan.job_id,plan.budget_id,p?.test_owner_id,p?.service_id,p?.request_id,p?.accounting_namespace];
 if(ids.some(id=>!uuid.test(id??'')||id===archived)||new Set(ids).size!==ids.length)blockers.push('invalid_or_reused_evaluation_identifiers');
 if(typeof p?.job_expires_at!=='string'||!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{6}Z$/.test(p.job_expires_at))blockers.push('unbound_job_expiration');
 if(p?.memory_policy!=='none'||p?.authorization_state!=='proposal_only'||JSON.stringify(p?.approved_tools)!==JSON.stringify(['inspect_input','extract_evidence','check_requirements']))blockers.push('changed_tool_or_memory_scope');
 const task={objective:fixture.objective,material:fixture.material,requirements:fixture.requirements};
 try{if(JSON.stringify(canonical(JSON.parse(p.task_input)))!==JSON.stringify(canonical(task))||hash(p.task_input)!==plan.input_sha256)blockers.push('changed_task_input');}catch{blockers.push('invalid_task_input');}
 if(fixture.id!=='supplied-synthesis'||plan.fixture_id!==fixture.id||plan.model!=='gpt-6-luna'||plan.max_requests!==1||plan.max_input_tokens!==24192||plan.max_output_tokens!==1024||plan.max_cost_microusd!==4000||plan.budget_limit_microusd!==5000||plan.use_memory!==false||plan.api_origin!=='http://127.0.0.1:55682')blockers.push('changed_model_or_execution_limits');
 return {scope_valid:blockers.length===0,blockers,scope_sha256:hash(JSON.stringify(canonical({plan,fixture}))),input_sha256:plan.input_sha256,budget_state:'proposal_only',paid_requests:0};
}
export async function credentials(env=process.env,path=resolve(root,'.env.eval')){
 let values={},loaded=false;
 try{const s=await lstat(path);if(!s.isFile()||s.isSymbolicLink()||s.uid!==process.getuid()||(s.mode&0o077)!==0||s.size>65536)throw Error('unsafe_private_environment');values=parseEnv(await readFile(path,'utf8'));loaded=true;}catch(e){if(e.code!=='ENOENT')throw Error('unsafe_private_environment');}
 const effective={...values,...env},key=effective.OPENAI_API_KEY??'',project=effective.OPENAI_PROJECT_ID??'';
 const report={private_file_loaded:loaded,key_present:!!key,key_basic_format_valid:key.startsWith('sk-')&&/^[!-~]+$/.test(key)&&!/["']/.test(key),key_source:env.OPENAI_API_KEY!==undefined?'process_environment':values.OPENAI_API_KEY?'owner_only_private_file':null,project_id:/^proj_[A-Za-z0-9_-]{1,114}$/.test(project)?project:null,project_format_valid:/^proj_[A-Za-z0-9_-]{1,114}$/.test(project),model_metadata_access_verified:false,project_billing_verified:false};
 // Never serialize or log this private return value. Only report may be emitted.
 return {key,project,report};
}
export async function databaseCheck(plan,{forExecution=false,execute=exec}={}){
 const p=plan.preflight,db=p.database;
 if(![plan.agent_id,plan.job_id,plan.budget_id,p.test_owner_id].every(id=>uuid.test(id??''))||!/^supa_m10a_[0-9a-f]{12}_test$/.test(db.name)||!/^\/tmp\/supa-m10a-pg-[0-9a-f]{12}\/s$/.test(db.socket_directory)||db.port!==55681)throw Error('invalid_database_target');
 const directory=await lstat(db.data_directory.replace(/\/data$/,''));
 if(!directory.isDirectory()||directory.isSymbolicLink()||directory.uid!==process.getuid()||(directory.mode&0o077)!==0)throw Error('unsafe_disposable_directory');
 const query=`SELECT json_build_object('database',current_database(),'data_directory',current_setting('data_directory'),'listen_addresses',current_setting('listen_addresses'),'system_identifier',(SELECT system_identifier::text FROM pg_control_system()),'epochs',(SELECT count(*) FROM application_epoch),'agents',(SELECT count(*) FROM agents),'agent',(SELECT data FROM agents WHERE id='${plan.agent_id}'),'owner',(SELECT principal_id FROM agent_access WHERE agent_id='${plan.agent_id}' AND role='owner'),'grants',(SELECT count(*) FROM agent_access),'job',(SELECT json_build_object('agent_id',agent_id,'customer_id',customer_id,'revision',revision,'state',state,'network',network,'input_sha256',encode(sha256(convert_to(input,'UTF8')),'hex'),'received_atoms',received_atoms,'earned_atoms',earned_atoms,'expires_at',to_char(expires_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS.US"Z"'),'not_expired',expires_at>clock_timestamp()) FROM economic_jobs WHERE id='${plan.job_id}'),'jobs',(SELECT count(*) FROM economic_jobs),'live_sessions',(SELECT count(*) FROM live_sessions),'inference_calls',(SELECT count(*) FROM inference_calls),'cycles',(SELECT count(*) FROM cycles),'actions',(SELECT count(*) FROM actions),'schedules',(SELECT count(*) FROM schedule_authorizations),'economic_entries',(SELECT count(*) FROM economic_entries),'budgets',(SELECT count(*) FROM live_budgets),'budget',(SELECT json_build_object('id',id,'limit_microusd',limit_microusd,'committed_microusd',committed_microusd) FROM live_budgets WHERE id='${plan.budget_id}'),'migrations',(SELECT json_agg(json_build_object('version',version,'success',success,'checksum',encode(checksum,'hex')) ORDER BY version) FROM _sqlx_migrations))`;
 const result=await execute('/usr/lib/postgresql/16/bin/psql',['-X','-qAt','--no-password','-v','ON_ERROR_STOP=1','-c',`BEGIN READ ONLY; ${query}; COMMIT;`],{env:{PATH:'/usr/bin:/bin',PGHOST:db.socket_directory,PGPORT:String(db.port),PGDATABASE:db.name,PGUSER:userInfo().username,PGPASSFILE:'/dev/null',PGSERVICEFILE:'/dev/null',PGCONNECT_TIMEOUT:'3',PGOPTIONS:'-c default_transaction_read_only=on -c statement_timeout=5000 -c lock_timeout=1000'},timeout:8000,maxBuffer:128000});
 const s=JSON.parse(result.stdout.trim());
 if(s.database!==db.name||s.data_directory!==db.data_directory||s.listen_addresses!==''||s.system_identifier!==db.system_identifier||s.epochs!==0||s.agents!==1||s.jobs!==1||s.grants!==1||s.owner!==p.test_owner_id||s.agent?.lifecycle!=='paused'||s.agent?.memory?.length!==0||s.job?.agent_id!==plan.agent_id||s.job?.customer_id!==p.test_owner_id||s.job?.revision!==0||s.job?.network!=='simulated'||s.job?.state!=='created'||s.job?.input_sha256!==plan.input_sha256||s.job?.received_atoms!==0||s.job?.earned_atoms!==0||s.job?.expires_at!==p.job_expires_at||!s.job?.not_expired||['live_sessions','inference_calls','cycles','actions','schedules','economic_entries'].some(k=>s[k]!==0))throw Error('disposable_state_mismatch_or_consumed_scope');
 if(s.budgets!==0&&!(s.budgets===1&&s.budget?.id===plan.budget_id&&s.budget.limit_microusd===5000&&s.budget.committed_microusd===0))throw Error('unexpected_accounting_state');
 if(forExecution&&s.budgets!==1)throw Error('independent_budget_provisioning_requires_approval');
 for(const m of s.migrations){const file=(await readdir(resolve(root,'migrations'))).find(f=>Number(f.split('_')[0])===m.version);if(!file||!m.success||createHash('sha384').update(await readFile(resolve(root,'migrations',file))).digest('hex')!==m.checksum)throw Error('migration_checksum_mismatch');}
 const files=(await readdir(resolve(root,'migrations'))).filter(f=>/^\d+_.+\.sql$/.test(f));if(files.length!==s.migrations.length)throw Error('migration_inventory_mismatch');
 return {database_verified:true,socket_only:true,epochs:0,agent_id:plan.agent_id,lifecycle:s.agent.lifecycle,job_id:plan.job_id,job_state:s.job.state,expires_at:s.job.expires_at,input_sha256:s.job.input_sha256,migrations_verified:s.migrations.length,budget_state:s.budgets?'installed_zero_spent':'proposal_only',live_requests:0,earned_revenue:0,scheduling_authorizations:0};
}
async function verifyModel(plan,privateConfig){
 if(!privateConfig.report.key_basic_format_valid||!privateConfig.report.project_format_valid||privateConfig.project!==plan.project_id)throw Error('verified_intended_project_configuration_required');
 return new Promise(resolveResult=>{
  const request=https.get('https://api.openai.com/v1/models/gpt-6-luna',{headers:{Authorization:`Bearer ${privateConfig.key}`,'OpenAI-Project':privateConfig.project},timeout:15000,agent:new https.Agent({keepAlive:false})},response=>{
   let bytes=0,body='';response.on('data',chunk=>{bytes+=chunk.length;if(bytes>16384){response.destroy();return;}body+=chunk;});response.on('error',()=>resolveResult({metadata_get_attempts:1,result:'bounded_transport_error',generation_requests:0}));response.on('end',()=>{let value;try{value=JSON.parse(body);}catch{}const verified=response.statusCode===200&&value?.id==='gpt-6-luna'&&value?.object==='model';resolveResult({metadata_get_attempts:1,http_status:response.statusCode,model_metadata_access_verified:verified,intended_project_header_accepted:verified,project_billing_verified:false,generation_requests:0});});
  });request.on('timeout',()=>request.destroy());request.on('error',()=>resolveResult({metadata_get_attempts:1,result:'transport_error',generation_requests:0}));
 });
}
export async function main(args=process.argv.slice(2)){
 const allowed=new Set(['--scope','--plan-stdin','--credentials','--check-database','--for-execution','--verify-project']);
 const pathIndex=args.indexOf('--scope');for(let i=0;i<args.length;i++){if(i===pathIndex+1&&pathIndex>=0)continue;if(!allowed.has(args[i]))throw Error('unsupported_preflight_argument');}
 const plan=args.includes('--plan-stdin')?JSON.parse(await readFile('/dev/stdin','utf8')):JSON.parse(await readFile(pathIndex<0?resolve(root,'config/live-work-evaluation-10a.json'):args[pathIndex+1],'utf8'));
 const fixture=JSON.parse(await readFile(resolve(root,'config/work-evaluation-fixtures.json'),'utf8')).cases.find(c=>c.id==='supplied-synthesis');
 const report=scopeCheck(plan,fixture);if(!report.scope_valid)throw Error('invalid_frozen_scope');
 if(args.includes('--credentials')||args.includes('--verify-project')){const c=await credentials();report.credentials=c.report;if(args.includes('--verify-project'))report.provider=await verifyModel(plan,c);}
 if(args.includes('--check-database')){report.database=await databaseCheck(plan,{forExecution:args.includes('--for-execution')});report.budget_state=report.database.budget_state;}
 report.ready_for_paid=false;report.authorization='fresh_operator_paid_approval_required';report.project_access_verified=report.provider?.model_metadata_access_verified===true;
 console.log(JSON.stringify(report,null,2));
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href)main().catch(()=>{console.error(JSON.stringify({error:'preflight_failed_closed_no_private_details',paid_requests:0}));process.exitCode=1;});
