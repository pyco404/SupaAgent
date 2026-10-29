#!/usr/bin/env node
/** Read-only local preflight. No model transport, migrations, reservation settlement or launch path. */
import {readFile,lstat,readdir} from 'node:fs/promises';
import {parseEnv} from 'node:util';
import {createHash} from 'node:crypto';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
const run=promisify(execFile),root=resolve(import.meta.dirname,'..');
export const budgetId='ad91e475-f1da-495a-8247-139643d2d8b1';
export const session={agent_id:null,request_id:null,consented_at:null,expires_at:null,model:'gpt-6-luna',max_requests:1,max_input_tokens:24192,max_output_tokens:1024,max_cost_microusd:5000,budget_id:budgetId,cumulative_ceiling_microusd:100000,authorize_paid:false,continuous_live:false,tools:['read_memory'],goal:'Compare safe ways to review the available memories, identify evidence gaps, and propose at most one read_memory action. Do not claim completion or modify data.'};
export function safeConnection(raw){const u=new URL(raw);if(!['postgres:','postgresql:'].includes(u.protocol)||!['127.0.0.1','localhost','[::1]'].includes(u.hostname)||u.search||u.hash||u.pathname.length<2)throw Error('local_database_configuration_required');return{host:u.hostname.replace(/^\[|\]$/g,''),port:u.port||'5432',database:decodeURIComponent(u.pathname.slice(1)),user:decodeURIComponent(u.username),password:decodeURIComponent(u.password)};}
async function configuration(env){
 const path=env.SUPA_EVAL_ENV_FILE===undefined?resolve(root,'.env.eval'):env.SUPA_EVAL_ENV_FILE;let values={};let loaded=false;
 if(path){try{const info=await lstat(path);if(!info.isFile()||info.isSymbolicLink()||(info.mode&0o077)!==0||info.uid!==process.getuid())throw Error('unsafe_private_environment');values=parseEnv(await readFile(path,'utf8'));loaded=true;}catch(e){if(e.code!=='ENOENT')throw Error('unsafe_private_environment');}}
 return{values:{...values,...env},loaded,shellKey:!!env.OPENAI_API_KEY,fileKey:!!values.OPENAI_API_KEY,override:!!env.OPENAI_API_KEY&&!!values.OPENAI_API_KEY,keyMatches:env.OPENAI_API_KEY&&values.OPENAI_API_KEY?env.OPENAI_API_KEY===values.OPENAI_API_KEY:null};
}
async function archivedBudget(){
 let committed=0,unknown=0,snapshots=0;try{for(const file of await readdir(resolve(root,'artifacts/live-eval'))){if(!file.startsWith(budgetId)||!file.endsWith('.persistence.json'))continue;try{const d=JSON.parse(await readFile(resolve(root,'artifacts/live-eval',file),'utf8'));if(d.budget?.id!==budgetId||d.budget.limit_microusd!==100000||!Number.isSafeInteger(d.budget.committed_microusd))continue;snapshots++;committed=Math.max(committed,d.budget.committed_microusd);unknown=Math.max(unknown,(d.budget_calls??[]).filter(c=>['unknown','reserved'].includes(c.status)).reduce((sum,c)=>sum+(Number.isSafeInteger(c.accounted_microusd)?c.accounted_microusd:0),0));}catch{}}}catch{}
 return{source:'preserved_local_snapshots_not_current_database_or_invoice',snapshots,last_observed_committed_microusd:committed,unknown_accounted_microusd:unknown};
}
export async function check({env=process.env,query}={}){
 const report={ready_for_paid:false,paid_authorization_supplied:false,generation_requests:0,provider_requests:0,database_changed:false,schema_changed:false,reservations_changed:false,configuration:{},database:{connected:false},prepared_session:session,blockers:[]};
 report.archived_budget=await archivedBudget();
 let c;try{c=await configuration(env);}catch{report.blockers.push('unsafe_private_environment');return report;}
 const v=c.values,key=v.OPENAI_API_KEY;
 report.configuration={environment_file_loaded:c.loaded,key_present:!!key,key_format_valid:!!key&&key.startsWith('sk-')&&/^[\x21-\x7e]+$/.test(key)&&!/["']/.test(key),shell_key_present:c.shellKey,file_key_present:c.fileKey,shell_key_overrides_file:c.override,shell_key_matches_file:c.keyMatches,project_configured:!!v.OPENAI_PROJECT_ID,project_format_valid:/^proj_[a-zA-Z0-9_-]{1,115}$/.test(v.OPENAI_PROJECT_ID??''),configured_budget_matches:v.SUPA_LIVE_BUDGET_ID===budgetId,configured_ceiling_matches:v.SUPA_LIVE_BUDGET_MICROUSD==='100000',prepared_output_limit_configured:v.SUPA_MAX_OUTPUT_TOKENS==='1024',admin_credentials_present:!!(v.OPENAI_ADMIN_KEY||v.OPENAI_ADMIN_API_KEY),explicit_model_configured:!!v.OPENAI_MODEL,model:['gpt-6-luna','gpt-6.1-sol'].includes(v.OPENAI_MODEL)?v.OPENAI_MODEL:null,controlled_inference_enabled:v.SUPA_CONTROLLED_INFERENCE==='true',live_opt_in:v.SUPA_ALLOW_OPENAI==='true',api_scheduler_enabled:v.SUPA_API_SCHEDULER_ENABLED==='true',project_spending_limits_verified:false,model_access_verified:false,billing_access_verified:false};
 if(!report.configuration.key_format_valid)report.blockers.push('missing_or_malformed_api_key');
 if(!report.configuration.project_format_valid)report.blockers.push('expected_project_not_configured');
 if(!report.configuration.configured_budget_matches||!report.configuration.configured_ceiling_matches)report.blockers.push('existing_budget_identity_and_ceiling_must_be_explicit');
 if(!report.configuration.prepared_output_limit_configured)report.blockers.push('prepared_output_limit_must_be_explicit');
 if(v.OPENAI_MODEL!==session.model)report.blockers.push('explicit_luna_model_required_for_prepared_session');
 if(v.SUPA_API_SCHEDULER_ENABLED==='true'||v.SUPA_PROVIDER&&v.SUPA_PROVIDER!=='offline')report.blockers.push('continuous_or_global_live_provider_must_be_disabled');
 let connection;try{connection=safeConnection(v.SUPA_READINESS_DATABASE_URL??v.SUPA_EVAL_DATABASE_URL??'');}catch{report.blockers.push('local_database_configuration_required');return report;}
 report.database={connected:false,host:connection.host,port:connection.port,name:connection.database,scope:v.SUPA_READINESS_DATABASE_URL?'explicit_read_only_override':'configured_evaluation_database',test_database:connection.database.endsWith('_test')};
 const execute=query??(async sql=>{const result=await run('psql',['-X','-qAt','--no-password','-v','ON_ERROR_STOP=1','-c',`BEGIN READ ONLY; ${sql}; COMMIT;`],{env:{PATH:env.PATH,PGHOST:connection.host,PGPORT:connection.port,PGDATABASE:connection.database,PGUSER:connection.user,PGPASSWORD:connection.password,PGCONNECT_TIMEOUT:'3',PGAPPNAME:'supa-readiness-read-only',PGOPTIONS:'-c default_transaction_read_only=on -c statement_timeout=5000 -c lock_timeout=1000'},timeout:8000,maxBuffer:128000});return JSON.parse(result.stdout.trim());});
 try{
  const tables=await execute("SELECT coalesce(json_agg(tablename),'[]'::json) FROM pg_tables WHERE schemaname=current_schema()");report.database.connected=true;
  const required=['agents','agent_access','schedule_authorizations','inference_calls','live_budgets','cycles','actions','live_sessions','_sqlx_migrations'];
  report.database.missing_tables=required.filter(t=>!tables.includes(t));
  if(report.database.missing_tables.length){report.blockers.push('schema_review_required_no_migrations_applied');return report;}
  const migrations=await execute("SELECT coalesce(json_agg(json_build_object('version',version,'success',success,'checksum',encode(checksum,'hex')) ORDER BY version),'[]'::json) FROM _sqlx_migrations");
  const expected=[];for(const name of (await readdir(resolve(root,'migrations'))).filter(n=>/^\d+_.+\.sql$/.test(n))){expected.push({version:Number(name.split('_')[0]),checksum:createHash('sha384').update(await readFile(resolve(root,'migrations',name))).digest('hex')});}
  report.database.migrations=migrations.map(m=>({version:m.version,success:m.success,checksum_matches:expected.some(e=>e.version===m.version&&e.checksum===m.checksum)}));
  report.database.migrations_current=expected.length===migrations.length&&report.database.migrations.every(m=>m.success&&m.checksum_matches);
  if(!report.database.migrations_current)report.blockers.push('migration_review_required_no_migrations_applied');
  report.database.inventory=await execute("SELECT json_build_object('agents',(SELECT count(*) FROM agents),'owned_agents',(SELECT count(DISTINCT agent_id) FROM agent_access WHERE role='owner'),'memories',(SELECT coalesce(sum(jsonb_array_length(data->'memory')),0) FROM agents),'authorizations',(SELECT count(*) FROM schedule_authorizations),'cycles',(SELECT count(*) FROM cycles),'actions',(SELECT count(*) FROM actions),'live_sessions',(SELECT count(*) FROM live_sessions))");
  report.database.budget=await execute(`SELECT coalesce((SELECT json_build_object('id',id,'limit_microusd',limit_microusd,'committed_microusd',committed_microusd,'remaining_microusd',greatest(limit_microusd-committed_microusd,0)) FROM live_budgets WHERE id='${budgetId}'),'null'::json)`);
  report.database.uncertain_calls=await execute(`SELECT json_build_object('legacy',(SELECT count(*) FROM inference_calls WHERE budget_id='${budgetId}' AND status IN ('reserved','unknown')),'controlled',(SELECT count(*) FROM live_sessions WHERE budget_id='${budgetId}' AND status IN ('reserved','unknown')))`);
  if(!report.database.budget)report.blockers.push('historical_budget_missing_do_not_recreate');
  else if(report.database.budget.limit_microusd!==100000||report.database.budget.remaining_microusd<3536)report.blockers.push('historical_budget_ceiling_or_headroom_invalid');
  if(report.database.budget&&report.database.budget.committed_microusd<report.archived_budget.last_observed_committed_microusd)report.blockers.push('database_commitment_below_archived_evidence_review_required');
  if(report.database.uncertain_calls.legacy||report.database.uncertain_calls.controlled)report.blockers.push('uncertain_charges_require_provider_evidence_no_refund_or_replay');
 }catch{report.blockers.push(report.database.connected?'database_audit_incomplete':'historical_database_unavailable');}
 report.blockers.push('project_limits_and_billing_require_verification','fresh_operator_paid_consent_required');return report;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href){if(process.argv.slice(2).some(a=>a!=='--check')){console.error(JSON.stringify({error:'read_only_check_only',generation_requests:0}));process.exitCode=2;}else{const report=await check();console.log(JSON.stringify(report,null,2));process.exitCode=report.blockers.length?2:0;}}
