#!/usr/bin/env node
/** Disposable local services only. Never imports dotenv, retained evaluations or primary state. */
import {randomBytes,randomUUID,createHash} from 'node:crypto';
import {spawn,execFileSync} from 'node:child_process';
import {mkdir,readFile,writeFile,open,stat,access} from 'node:fs/promises';
import {resolve} from 'node:path';
const root=resolve(import.meta.dirname,'..'),out=resolve(root,'artifacts/local-launch'),file=resolve(out,'runtime.json'),pg='/usr/lib/postgresql/16/bin/';
const env={PATH:process.env.PATH,HOME:process.env.HOME,LANG:'C.UTF-8'},command=process.argv[2]??'status';
const hex=()=>randomBytes(32).toString('hex');
const digest=data=>createHash('sha256').update(data).digest('hex');
async function save(c){await writeFile(file,JSON.stringify(c,null,2)+'\n',{mode:0o600});}
function sql(c,query,db='postgres'){return execFileSync(pg+'psql',['-XqAt','--no-password','-v','ON_ERROR_STOP=1'],{env:{...env,PGHOST:c.socket,PGPORT:'55690',PGUSER:'pikoo',PGDATABASE:db,PGPASSFILE:'/dev/null',PGSERVICEFILE:'/dev/null',PGOPTIONS:'-c log_statement=none -c log_min_error_statement=panic'},input:query,stdio:['pipe','pipe','pipe'],encoding:'utf8'}).trim();}
async function load(){const s=await stat(file);if(s.uid!==process.getuid()||(s.mode&0o077))throw Error();const c=JSON.parse(await readFile(file,'utf8'));if(c.purpose!=='disposable_local_agent_launch'||!/^supa_launch_[a-f0-9]{12}_test$/.test(c.database)||!c.base.startsWith('/tmp/supa-launch-')||c.socket!==c.base+'/socket')throw Error();return c;}
async function halted(pid,marker){if(!pid)return;let line;try{line=await readFile(`/proc/${pid}/cmdline`,'utf8');}catch{return;}if(!line.includes(marker))throw Error('process_identity_mismatch');process.kill(pid,'SIGTERM');for(let i=0;i<100;i++){try{process.kill(pid,0);}catch{return;}await new Promise(r=>setTimeout(r,100));}throw Error('graceful_shutdown_timeout');}
async function start(c){
 if(c.expires_at<=Date.now())throw Error('demo_scope_expired_no_automatic_renewal');
 if(sql(c,'SHOW data_directory')!==c.base+'/data'||sql(c,'SHOW listen_addresses')!==''||sql(c,'SELECT system_identifier::text FROM pg_control_system()')!==c.system_identifier)throw Error('wrong_disposable_cluster');
 if(digest(await readFile(c.binary))!==c.binary_sha256)throw Error('reviewed_binary_changed');
 const log=await open(resolve(out,'services.log'),'a',0o600);
 const credentials=c.accounts.map(({id,role,token})=>({id,role,token}));
 const common={...env,DATABASE_URL:`postgresql://supa_launch_app@localhost/${c.database}?host=${c.socket}&port=55690`,PGPASSWORD:c.database_password,SUPA_API_TOKEN:c.service_token,SUPA_PROVIDER:'offline',SUPA_ALLOW_OPENAI:'false',SUPA_ALLOW_ANTHROPIC:'false',SUPA_CONTROLLED_INFERENCE:'false',SUPA_CONTROLLED_WORK:'false',SUPA_WORKER_ENABLED:'false',SUPA_WALLET_ENABLED:'false',SUPA_SITE_ENV_FILE:'',SUPA_BACKEND_URL:c.backend_origin};
 const api=spawn(c.binary,[],{cwd:root,env:{...common,SUPA_BIND:'127.0.0.1:8080',SUPA_API_SCHEDULER_ENABLED:'true',SUPA_OPERATOR_CREDENTIALS:JSON.stringify(credentials),SUPA_OPERATOR_SESSION_DEADLINES:JSON.stringify(Object.fromEntries(c.accounts.map(a=>[a.id,Math.floor(c.expires_at/1000)])))},detached:true,stdio:['ignore',log.fd,log.fd]});api.unref();c.api_pid=api.pid;await save(c);
 async function ready(url,pid){for(let i=0;i<100;i++){try{process.kill(pid,0);if((await fetch(url,{signal:AbortSignal.timeout(500)})).ok)return;}catch{}await new Promise(r=>setTimeout(r,100));}throw Error('disposable_service_not_ready');}
 await ready(c.backend_origin+'/ready',api.pid);
 const web=spawn(process.execPath,['server.mjs'],{cwd:resolve(root,'reference-web'),env:{...env,PORT:'3200',SUPA_API_TOKEN:c.service_token,SUPA_SITE_ENV_FILE:'',SUPA_BACKEND_URL:c.backend_origin,SUPA_LOCAL_LAUNCH_SCOPE:file,SUPA_WEB_OPERATORS:JSON.stringify(c.accounts.map(a=>({username:a.username,password:a.password,api_token:a.token})))},detached:true,stdio:['ignore',log.fd,log.fd]});web.unref();c.web_pid=web.pid;await save(c);await ready('http://127.0.0.1:3200/healthz',web.pid);await log.close();
}
async function main(){
 try{await access(resolve(root,'postgres-data/infrastructure-v1/data/postmaster.pid'));throw Error('primary_must_remain_stopped');}catch(error){if(error.code!=='ENOENT')throw error;}
 if(command==='init'){
  await mkdir(out,{recursive:true,mode:0o700});try{await access(file);throw Error('existing_demo_must_not_be_replaced');}catch(e){if(e.code!=='ENOENT')throw e;}
  const run=randomUUID().replaceAll('-','').slice(0,12),base='/tmp/supa-launch-'+run;
  await mkdir(base,{mode:0o700});await mkdir(base+'/socket',{mode:0o700});
  execFileSync(pg+'initdb',['-D',base+'/data','--auth-local=peer','--auth-host=reject','--encoding=UTF8','--locale=C.UTF-8'],{env,stdio:'ignore'});
  const conf=await readFile(base+'/data/postgresql.conf','utf8');await writeFile(base+'/data/postgresql.conf',conf+`\nlisten_addresses=''\nunix_socket_directories='${base}/socket'\nunix_socket_permissions=0700\nport=55690\nlog_statement='none'\nlog_min_error_statement=panic\nfsync=on\nsynchronous_commit=on\n`);
  await writeFile(base+'/data/pg_hba.conf','local all pikoo peer\nlocal all supa_launch_app scram-sha-256\nlocal all all reject\n');
  execFileSync(pg+'pg_ctl',['-D',base+'/data','-l',base+'/postgres.log','-w','start'],{env,stdio:'ignore'});
  const account=(role,username=role)=>({id:randomUUID(),role,username:'launch-'+username,password:hex(),token:hex()});
  const binary=resolve(root,'target/work-review-validation/debug/supa-api');
  const c={purpose:'disposable_local_agent_launch',primary_mutations_authorized:false,base,socket:base+'/socket',database:'supa_launch_'+run+'_test',database_password:hex(),service_token:hex(),backend_origin:'http://127.0.0.1:8080',expires_at:Date.now()+8*3600000,binary,binary_sha256:digest(await readFile(binary)),accounts:[account('owner'),account('operator'),account('viewer'),account('owner','outsider')]};
  sql(c,`CREATE ROLE supa_launch_app LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '${c.database_password}'; CREATE DATABASE ${c.database} OWNER supa_launch_app;`);
  c.system_identifier=sql(c,'SELECT system_identifier::text FROM pg_control_system()');await save(c);await start(c);
 }else{
  const c=await load();
  if(command==='restart'){await halted(c.web_pid,'server.mjs');await halted(c.api_pid,c.binary);await start(c);}
  else if(command==='stop'){await halted(c.web_pid,'server.mjs');await halted(c.api_pid,c.binary);c.web_pid=null;c.api_pid=null;await save(c);}
  else if(command!=='status')throw Error('expected_init_restart_stop_status');
 }
 const c=await load();console.log(JSON.stringify({site:'http://127.0.0.1:3200',mode:'disposable_offline',database:c.database,expires_at:new Date(c.expires_at).toISOString(),authentication_file:'artifacts/local-launch/runtime.json',paid_inference:false,primary_touched:false}));
}
main().catch(error=>{console.error('local_launch_demo_failed_closed: '+(error.message?.match(/^[a-z_]+$/)?error.message:'inspect_private_service_log'));process.exitCode=1;});
