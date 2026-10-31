#!/usr/bin/env node
/** Synthetic onboarding only. No primary epoch, real credentials or paid provider transport. Workers are isolated fixtures only. */
import assert from 'node:assert/strict';
import {randomBytes,randomUUID} from 'node:crypto';
import {spawn,execFileSync} from 'node:child_process';
import {createServer} from 'node:net';
import {readFile,writeFile,lstat} from 'node:fs/promises';
import {resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from '../web/node_modules/playwright-core/index.mjs';

export async function stopFixture(child, timeoutMs=10000) {
  if(!child||child.exitCode!==null||child.signalCode!==null)return;
  let timer;
  const exited=new Promise(resolve=>child.once('exit',resolve));
  child.kill('SIGTERM');
  try {
    const graceful=await Promise.race([exited.then(()=>true),new Promise(resolve=>{timer=setTimeout(()=>resolve(false),timeoutMs);})]);
    if(!graceful){child.kill('SIGKILL');await exited;throw Error('fixture_forced_shutdown');}
  } finally {clearTimeout(timer);}
}

async function main(){
const root=resolve(import.meta.dirname,'..');
let database;try{database=new URL(process.env.TEST_DATABASE_URL??'invalid:');}catch{throw Error('dedicated_disposable_database_required');}
assert.equal(database.protocol,'postgresql:');
assert.ok(database.hostname==='localhost'&&database.username==='pikoo'&&database.password==='', 'passwordless_fixed_local_test_identity_required');
assert.equal(database.pathname,'/supa_cutover_test_onboarding_test');
assert.deepEqual([...database.searchParams.keys()].sort(),['host','port']);assert.equal(database.searchParams.get('port'),'55452');
const socket=resolve(database.searchParams.get('host')),base=dirname(socket);
assert.equal(dirname(base),resolve(root,'postgres-data/onboarding-validation'));assert.equal(socket,resolve(base,'socket'));
const scope=JSON.parse(await readFile(resolve(base,'scope.json'),'utf8'));
assert.equal(scope.purpose,'disposable_milestone_8');assert.equal(scope.primary_mutations_authorized,false);
for(const path of [base,socket]){const s=await lstat(path);assert.ok(s.isDirectory()&&!s.isSymbolicLink()&&(s.mode&0o077)===0);}
const pgEnv={PATH:process.env.PATH,HOME:process.env.HOME,PGHOST:socket,PGPORT:'55452',PGDATABASE:'supa_cutover_test_onboarding_test',PGUSER:'pikoo',PGPASSFILE:'/dev/null',PGSERVICEFILE:'/dev/null'};
function sql(query){return execFileSync('/usr/lib/postgresql/16/bin/psql',['-XqAt','-v','ON_ERROR_STOP=1','-c',query],{env:pgEnv,stdio:['ignore','pipe','ignore'],encoding:'utf8'}).trim();}
assert.equal(sql('SHOW data_directory'),resolve(base,'data'));assert.equal(sql('SHOW listen_addresses'),'');
assert.equal(sql('SELECT count(*) FROM application_epoch'),'0');assert.equal(sql('SELECT count(*) FROM agents'),'0');
const evidence=resolve(process.env.SUPA_ONBOARDING_EVIDENCE_DIR??'');
assert.ok(evidence.startsWith(resolve(root,'artifacts/milestone-8')+'/'));
const plan=JSON.parse(await readFile(resolve(root,'config/first-agent-plan.json'),'utf8'));
assert.equal(plan.approved,false);assert.equal(plan.agent_id,null);assert.equal(plan.worker_launch_authorized,false);
async function port(){const s=createServer();await new Promise(r=>s.listen(0,'127.0.0.1',r));const p=s.address().port;await new Promise(r=>s.close(r));return p;}
const apiPort=await port(),webPort=await port(),api=`http://127.0.0.1:${apiPort}`,site=`http://127.0.0.1:${webPort}`;
const account=role=>({id:randomUUID(),role,token:randomBytes(40).toString('hex'),username:'fixture-'+role,password:randomBytes(30).toString('hex')});
let owner=account('owner');const viewer=account('viewer'),operator=account('operator'),other=account('owner');other.username='fixture-outsider';
assert.notEqual(owner.id,plan.proposed_owner_principal);
const serviceToken=randomBytes(40).toString('hex');
const common={PATH:process.env.PATH,HOME:process.env.HOME,DATABASE_URL:database.href,SUPA_API_TOKEN:serviceToken,SUPA_PROVIDER:'offline',SUPA_ALLOW_OPENAI:'false',SUPA_CONTROLLED_INFERENCE:'false',SUPA_API_SCHEDULER_ENABLED:'false',SUPA_WORKER_ENABLED:'false',SUPA_WALLET_ENABLED:'false',SUPA_SITE_ENV_FILE:'',SUPA_BACKEND_URL:api};
let service,web,browser;const children=[],logs=[],errors=[];
function launch(command,args,cwd,env){const child=spawn(command,args,{cwd,env:{...common,...env},stdio:['ignore','pipe','pipe']});for(const stream of [child.stdout,child.stderr])stream.on('data',chunk=>{if(logs.join('').length<2_000_000)logs.push(chunk.toString());});children.push(child);return child;}
async function ready(url,child){for(let n=0;n<120;n++){assert.equal(child.exitCode,null,'fixture_service_exited');try{if((await fetch(url,{signal:AbortSignal.timeout(500)})).ok)return;}catch{}await new Promise(r=>setTimeout(r,100));}throw Error('fixture_readiness_timeout');}
const halt=stopFixture;
async function start(accounts){
 service=launch(resolve(root,'target/debug/supa-api'),[],root,{SUPA_BIND:`127.0.0.1:${apiPort}`,SUPA_OPERATOR_CREDENTIALS:JSON.stringify(accounts.map(({id,role,token})=>({id,role,token})))});await ready(api+'/ready',service);
 web=launch(process.execPath,['server.mjs'],resolve(root,'reference-web'),{PORT:String(webPort),SUPA_WEB_OPERATORS:JSON.stringify(accounts.map(({username,password,token})=>({username,password,api_token:token})))});await ready(site+'/healthz',web);
}
async function request(path,body,user=owner,extra={}){const r=await fetch(api+path,{headers:{Authorization:`Bearer ${user.token}`,'Content-Type':'application/json',...extra},...(body===undefined?{}:{method:'POST',body:JSON.stringify(body)})});return {status:r.status,body:await r.json()};}
async function ok(path,body,user=owner){const r=await request(path,body,user);assert.equal(r.status,200,`fixture HTTP ${r.status} at ${path}`);return r.body;}
function denied(r){assert.ok([400,401,403,409,422].includes(r.status),'fixture_permission_or_state_denial_required');}
const basic=user=>'Basic '+Buffer.from(`${user.username}:${user.password}`).toString('base64');
async function capture(page,id,label,width){await page.setViewportSize({width,height:width===390?844:1000});await page.goto(site+'/operator');await page.goto(site+'/intelligence');await page.waitForFunction(id=>[...document.querySelector('#agent-select').options].some(o=>o.value===id),id);await page.locator('#agent-select').evaluate((el,id)=>{el.value=id;el.dispatchEvent(new Event('change'));},id);await page.locator('.buffer[data-typing-state=complete]').waitFor();await page.locator('.schedule-panel summary').click();assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:resolve(evidence,label+'.png')});}
try{
 await start([owner,viewer,operator,other]);
 const agent=await ok('/agents',plan.agent_request),id=agent.id,path=`/agents/${id}`;
 assert.equal(agent.lifecycle,'created');assert.equal(agent.config.scheduler_enabled,false);
 assert.deepEqual(agent.capabilities.slice().sort(),plan.agent_request.capabilities.slice().sort());
 const goals=[];for(const description of plan.goals)goals.push(await ok(path+'/goals',{description}));
 denied(await request(path+'/cycles',{}));assert.equal((await ok(path+'/cycles')).length,0);
 denied(await request(path+'/memory',undefined,other,{'x-supa-principal':owner.id,'x-supa-role':'owner'}));
 denied(await request('/agents',{...plan.agent_request,owner_id:owner.id},viewer));
 for(const user of [viewer,operator])await ok(path+'/access',{principal_id:user.id,role:user.role});
 await ok(path+'/memory',undefined,viewer);denied(await request(path+'/cycles',{},viewer));
 const balance=await ok(path+'/simulated-funding',{kind:'seed',amount_microusd:20000,idempotency_key:randomUUID(),simulated:true});assert.equal(balance.balance_microusd,20000);
 async function command(operation,user=owner,policy=null){const state=await ok(path+'/schedule');return request(path+'/schedule',{request_id:randomUUID(),expected_revision:state.revision,operation,authorize_offline:true,consented_at:new Date().toISOString(),reason:'Explicit disposable onboarding rehearsal',policy},user);}
 const policy={...plan.future_offline_policy,goal_ids:goals.map(goal=>goal.id),max_cycles:10,simulated_budget_microusd:11000,interval_seconds:5,valid_for_seconds:180};
 denied(await command('start',operator,policy));denied(await command('start',viewer,policy));
 assert.equal((await command('start',owner,policy)).status,200);
 await new Promise(r=>setTimeout(r,200));assert.equal((await ok(path+'/cycles')).length,0);
 await ok(path+'/cycles',{},operator);assert.equal((await command('pause',operator)).status,200);
 const memories=await ok(path+'/memory'),cycles=await ok(path+'/cycles'),financials=await ok(path+'/financials'),schedule=await ok(path+'/schedule');
 assert.equal(cycles.length,1);assert.ok(memories.length>0);assert.equal(schedule.lifecycle,'paused');assert.equal(schedule.reserved_microusd,1100);
 assert.equal(sql('SELECT count(*) FROM live_budgets'),'0');assert.equal(sql('SELECT count(*) FROM economic_entries'),'0');assert.equal(sql('SELECT count(*) FROM wallet_identities'),'0');
 const oldOwner=owner;owner={...owner,token:randomBytes(40).toString('hex'),password:randomBytes(30).toString('hex')};
 await halt(web);await halt(service);await start([owner,operator,other]);
 assert.equal((await request(path+'/memory',undefined,oldOwner)).status,401);assert.equal((await request(path+'/memory',undefined,viewer)).status,401);
 assert.equal((await fetch(site+'/api'+path+'/memory',{headers:{Authorization:basic(oldOwner)}})).status,401);
 assert.equal((await fetch(site+'/api'+path+'/memory',{headers:{Authorization:basic(viewer)}})).status,401);
 assert.deepEqual(await ok(path+'/memory'),memories);assert.deepEqual(await ok(path+'/cycles'),cycles);assert.deepEqual(await ok(path+'/financials'),financials);
 assert.equal((await ok(path+'/access')).role,'owner');assert.equal((await ok(path+'/schedule')).lifecycle,'paused');
 browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,...(process.env.SUPA_ONBOARDING_REA_PORT?{args:[`--remote-debugging-port=${Number(process.env.SUPA_ONBOARDING_REA_PORT)}`,'--remote-debugging-address=127.0.0.1']}:{})});
 const context=await browser.newContext({httpCredentials:owner,reducedMotion:'reduce',colorScheme:'dark'}),page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));
 await capture(page,id,'desktop-paused-owner',1440);await capture(page,id,'mobile-paused-owner',390);
 // The unsigned primary proposal stays at three cycles. This separately scoped
 // disposable authorization permits exactly ten; no real authorization is installed.
 const workerPort=await port();let worker;
 async function startWorker(){worker=launch(resolve(root,'target/debug/supa-runtime'),[],root,{SUPA_WORKER_ENABLED:'true',SUPA_WORKER_POLL_SECONDS:'1',SUPA_WORKER_BIND:`127.0.0.1:${workerPort}`});await ready(`http://127.0.0.1:${workerPort}/ready`,worker);}
 async function completed(){return Number(sql(`SELECT count(*) FROM cycles WHERE agent_id='${id}' AND status='completed'`));}
 async function waitCycles(count){for(let n=0;n<650;n++){assert.equal(worker.exitCode,null,'fixture_worker_exited');if(await completed()===count)return;await new Promise(r=>setTimeout(r,100));}throw Error('bounded_worker_cycle_timeout');}
 await startWorker();
 await new Promise(r=>setTimeout(r,1200));assert.equal(await completed(),1);
 assert.equal((await command('resume',operator)).status,200);
 await waitCycles(5);assert.equal((await command('pause',operator)).status,200);
 await halt(worker);await halt(web);await halt(service);await start([owner,operator,other]);
 assert.equal((await ok(path+'/schedule')).lifecycle,'paused');
 await startWorker();await new Promise(r=>setTimeout(r,1200));assert.equal(await completed(),5);
 assert.equal((await command('resume',operator)).status,200);
 await waitCycles(10);await new Promise(r=>setTimeout(r,6200));assert.equal(await completed(),10);
 await halt(worker);
 const exhausted=await ok(path+'/schedule');assert.equal(exhausted.lifecycle,'paused');assert.equal(exhausted.last_error,'schedule_limit_reached');assert.equal(exhausted.cycles_started,10);assert.equal(exhausted.reserved_microusd,11000);
 const workerCycles=Number(sql(`SELECT count(*) FROM cycles WHERE agent_id='${id}' AND worker_id IS NOT NULL`));assert.equal(workerCycles,9);
 assert.equal(sql(`SELECT count(*) FROM cycles WHERE agent_id='${id}' AND status='running'`),'0');
 assert.equal(sql("SELECT count(*) FROM worker_health WHERE kind='worker' AND stopped_at IS NOT NULL"),'2');
 const assessments=Number(sql(`SELECT count(*) FROM goal_evaluations WHERE agent_id='${id}'`));assert.ok(assessments>0);
 assert.ok((await ok(path+'/memory')).length>=memories.length);
 // New fixture-only consent tests expiry independently of the exhausted ceiling.
 assert.equal((await command('renew',owner,{...policy,max_cycles:1,simulated_budget_microusd:1100})).status,200);
 // Fixture clock injection only. Real authorizations are never altered by this script.
 sql(`UPDATE agent_schedules SET expires_at=now()-interval '1 second' WHERE agent_id='${id}'`);
 denied(await request(path+'/cycles',{},operator));
 const expired=await ok(path+'/schedule');assert.equal(expired.lifecycle,'paused');assert.equal(expired.last_error,'authorization_expired');assert.equal(expired.cycles_started,0);assert.equal(expired.reserved_microusd,0);
 await ok(path+'/emergency-stop',{reason:'Explicit disposable emergency stop'},operator);
 assert.equal((await ok(path+'/schedule')).lifecycle,'suspended');denied(await command('resume',operator));
 assert.ok((await ok(path+'/actions')).every(action=>action.authorization_outcome==='allowed'&&action.status==='succeeded'));
 assert.ok((await ok(path+'/inference')).every(call=>call.policy.live===false));
 assert.equal(sql('SELECT count(*) FROM live_sessions'),'0');assert.equal(sql('SELECT count(*) FROM economic_receipts'),'0');
 assert.deepEqual(errors,[]);
 for(const secret of [serviceToken,oldOwner.token,owner.token,viewer.token,operator.token,other.token,oldOwner.password,owner.password,viewer.password,...plan.goals])assert.ok(!logs.join('').includes(secret),'secret_or_private_goal_in_service_log');
 const report={passed:true,fixture_only:true,primary_epoch_connected:false,proposed_owner_installed:false,synthetic_owner_principal:owner.id,synthetic_agent_id:id,created_inactive:true,explicit_goal_count:goals.length,finite_policy:policy,offline_cycles:10,memories_after_first_cycle:memories.length,simulated_seed_microusd:20000,simulated_expenses_microusd:11000,real_spending:0,live_provider_requests:0,worker_processes_launched:2,background_cycles:workerCycles,goal_assessments:assessments,worker_restart_while_paused_verified:true,execution_ceiling_verified:true,role_boundaries_verified:true,forged_identity_denied:true,revoked_api_and_browser_credentials_denied:true,rotation_preserves_owner_and_records:true,pause_survives_restart:true,expiry_fixture_clock_injection:true,expired_authorization_denied:true,emergency_stop_fences_resume:true,final_lifecycle:'suspended',historical_accounting_imported:false,recipient:null,credentials_or_private_goals_in_logs:false,browser_errors:errors,viewports:[1440,390]};
 await writeFile(resolve(evidence,'onboarding.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx',mode:0o600});
 console.log(JSON.stringify({passed:true,fixture_only:true,offline_cycles:10,credential_revocation_verified:true}));
 if(process.env.SUPA_ONBOARDING_REA_PORT){await writeFile(resolve(evidence,'browser-ready.json'),JSON.stringify({origin:site,cdp_endpoint:`http://127.0.0.1:${process.env.SUPA_ONBOARDING_REA_PORT}`,url:page.url(),pid:process.pid})+'\n',{flag:'wx',mode:0o600});await new Promise(r=>{process.once('SIGTERM',r);process.once('SIGINT',r);});}
}finally{
 try{await browser?.close();}finally{
  const cleanup=await Promise.allSettled(children.map(child=>halt(child)));
  if(cleanup.some(result=>result.status==='rejected'))throw Error('fixture_cleanup_failed');
 }
}
}
if(resolve(process.argv[1]??'')===fileURLToPath(import.meta.url))main().catch(()=>{console.error('onboarding_fixture_failed_inspect_private_evidence');process.exitCode=1;});
