#!/usr/bin/env node
/** Actual PostgreSQL -> Rust -> Next.js -> browser check. Offline, disposable data only. */
import assert from 'node:assert/strict';
import { randomBytes, randomUUID } from 'node:crypto';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { chromium } from '../web/node_modules/playwright-core/index.mjs';

const root=resolve(import.meta.dirname,'..'),database=process.env.TEST_DATABASE_URL;
assert.ok(database,'TEST_DATABASE_URL required');
const parsed=new URL(database);
assert.ok(['127.0.0.1','localhost','[::1]'].includes(parsed.hostname)&&parsed.pathname.endsWith('_test'),'dedicated local *_test database required');
const evidence=process.env.SUPA_STAGING_EVIDENCE_DIR?resolve(process.env.SUPA_STAGING_EVIDENCE_DIR):await mkdtemp(resolve(tmpdir(),'supa-staging-'));
await mkdir(evidence,{recursive:true});
async function port(){const server=createServer();await new Promise(r=>server.listen(0,'127.0.0.1',r));const value=server.address().port;await new Promise(r=>server.close(r));return value;}
const apiPort=await port(),webPort=await port(),api=`http://127.0.0.1:${apiPort}`,site=`http://127.0.0.1:${webPort}`;
const token=randomBytes(32).toString('hex'),ownerToken=randomBytes(32).toString('hex'),otherToken=randomBytes(32).toString('hex'),ownerId=randomUUID(),otherId=randomUUID(),operator={username:'smoke-operator',password:randomBytes(24).toString('hex')},visitor={username:'smoke-viewer',password:randomBytes(24).toString('hex')};
const basic=person=>'Basic '+Buffer.from(`${person.username}:${person.password}`).toString('base64');
const env={PATH:process.env.PATH,HOME:process.env.HOME,NODE_ENV:'production',NEXT_TELEMETRY_DISABLED:'1',__NEXT_PROCESSED_ENV:'true',DATABASE_URL:database,SUPA_API_TOKEN:token,SUPA_PROVIDER:'offline',SUPA_API_SCHEDULER_ENABLED:'false',SUPA_ALLOW_OPENAI:'false',SUPA_WALLET_ENABLED:'false',SUPA_SITE_ENV_FILE:'',SUPA_SITE_MODE:'staging',SUPA_SITE_ORIGIN:site,SUPA_BACKEND_URL:api,SUPA_PRIVATE_STAGING:'true',SUPA_CONTROLLED_INFERENCE:'false',SUPA_OPERATOR_CREDENTIALS:JSON.stringify([{id:ownerId,role:'owner',token:ownerToken},{id:otherId,role:'viewer',token:otherToken}]),SUPA_WEB_OPERATORS:JSON.stringify([{...operator,api_token:ownerToken}]),SUPA_STAGING_USERNAME:visitor.username,SUPA_STAGING_PASSWORD:visitor.password};
const children=[],errors=[];let browser,agent;
function launch(command,args,cwd,extra={}){const child=spawn(command,args,{cwd,env:{...env,...extra},stdio:['ignore','ignore','ignore']});children.push(child);return child;}
async function ready(url,child){for(let i=0;i<120;i++){assert.equal(child.exitCode,null,'service exited');try{if((await fetch(url,{signal:AbortSignal.timeout(500)})).ok)return;}catch{}await new Promise(r=>setTimeout(r,150));}throw Error('service readiness timeout');}
async function request(path,body){const response=await fetch(api+path,{headers:{Authorization:`Bearer ${ownerToken}`,'Content-Type':'application/json'},...(body===undefined?{}:{method:'POST',body:JSON.stringify(body)})});assert.ok(response.ok,`Rust request failed: ${path} HTTP ${response.status}`);return response.json();}
async function frontend(path,person=operator){return fetch(site+path,{headers:person?{Authorization:basic(person)}:{},redirect:'manual'});}
try{
  const service=launch(resolve(root,'target/debug/supa-api'),[],root,{SUPA_BIND:`127.0.0.1:${apiPort}`});await ready(api+'/ready',service);
  const app=launch(process.execPath,['node_modules/next/dist/bin/next','start','--hostname','127.0.0.1','--port',String(webPort)],resolve(root,'web'));await ready(site+'/healthz',app);
  assert.equal((await frontend('/',null)).status,401);
  assert.equal((await frontend('/api/agents',visitor)).status,200);
  assert.equal((await frontend('/.env.eval',operator)).status,404);
  assert.equal((await frontend('/operator',visitor)).status,401);
  const made=await request('/agents',{name:'Milestone 5 isolated offline verification',capabilities:['read_memory','remember','evaluate_goal'],config:{scheduler_enabled:false,interval_seconds:1}});agent=made.id;
  const base=`/agents/${agent}`;
  const goal=await request(base+'/goals',{description:'Verify persistent integration without paid operations'});
  await request(base+'/simulated-funding',{kind:'seed',amount_microusd:10000,idempotency_key:randomUUID(),simulated:true});
  await request(base+'/schedule',{request_id:randomUUID(),expected_revision:0,operation:'start',authorize_offline:true,reason:'Explicit isolated deterministic integration test',policy:{goal_ids:[goal.id],interval_seconds:5,max_cycles:2,simulated_budget_microusd:2200,valid_for_seconds:300}});
  await request(base+'/cycles',{});
  const schedule=await request(base+'/schedule');
  await request(base+'/schedule',{request_id:randomUUID(),expected_revision:schedule.revision,operation:'pause',authorize_offline:true,reason:'Keep integration fixture paused',policy:null});
  const cycles=await request(base+'/cycles'),memory=await request(base+'/memory'),actions=await request(base+'/actions'),financials=await request(base+'/financials');
  assert.equal(cycles[0].status,'completed');assert.ok(memory.length);assert.ok(actions.some(action=>action.status==='succeeded'));assert.equal(financials.simulated,true);assert.ok(financials.inference_microusd>0);
  for(const deniedToken of [token,otherToken]) {
    const deniedRead=await fetch(api+base+'/memory',{headers:{Authorization:`Bearer ${deniedToken}`,'x-supa-principal':ownerId,'x-supa-role':'owner'}});
    assert.equal(deniedRead.status,403);
  }
  assert.equal((await frontend('/api'+base+'/memory',visitor)).status,401);
  assert.equal((await frontend('/api'+base+'/memory',operator)).status,200);
  const identities=await(await frontend('/api/agents',visitor)).json();assert.deepEqual(Object.keys(identities.find(row=>row.id===agent)).sort(),['created_at','id','lifecycle','name']);
  const denied=await fetch(site+'/api'+base+'/resume',{method:'POST',headers:{Authorization:basic(operator)}});assert.equal(denied.status,405);
  // Separate worker starts paused, even with an otherwise schedulable identity.
  const workerPort=await port();const worker=launch(resolve(root,'target/debug/supa-runtime'),[],root,{SUPA_WORKER_BIND:`127.0.0.1:${workerPort}`,SUPA_WORKER_ENABLED:'false'});await ready(`http://127.0.0.1:${workerPort}/ready`,worker);
  browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,...(process.env.SUPA_REA_CDP_PORT?{args:[`--remote-debugging-port=${Number(process.env.SUPA_REA_CDP_PORT)}`,'--remote-debugging-address=127.0.0.1']}:{})});
  const context=await browser.newContext({httpCredentials:operator,reducedMotion:'reduce',colorScheme:'dark'}),page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));
  assert.equal((await frontend('/api/autonomy',visitor)).status,401);
  await page.setViewportSize({width:1440,height:1000});
  await page.goto(site+'/site/#intelligence',{waitUntil:'domcontentloaded'});
  await page.getByText('Run an offline autonomy workflow',{exact:true}).click();
  const workflowGoal='Review saved outcomes and retain verified lessons';
  await page.getByLabel('Goal / operator-only').fill(workflowGoal);
  await page.screenshot({path:resolve(evidence,'offline-workflow-desktop.png')});
  await page.locator('.autonomy-panel').filter({hasText:'Run an offline autonomy workflow'}).getByRole('checkbox').check();
  const completion=page.waitForResponse(response=>response.url()===site+'/api/autonomy'&&response.request().method()==='POST');
  await page.getByRole('button',{name:'Run authorized offline workflow'}).click();
  const outcome=await(await completion).json();assert.equal(outcome.lifecycle,'paused');assert.equal(outcome.paid_requests,0);assert.equal(outcome.cycles.length,3);
  await page.locator('.autonomy-panel [role=status]').filter({hasText:'3 completed offline cycles'}).waitFor();
  const offlineBase=`/agents/${outcome.agent_id}`,offlineState=await request(offlineBase),offlineCycles=await request(offlineBase+'/cycles'),offlineActions=await request(offlineBase+'/actions'),offlineMemory=await request(offlineBase+'/memory');
  assert.equal(offlineState.agent.lifecycle,'paused');assert.equal(offlineState.agent.config.scheduler_enabled,false);assert.equal(offlineState.financials.balance_microusd,6700);assert.equal(offlineCycles.length,3);assert.ok(offlineMemory.length>=3);assert.ok(offlineActions.length>=8);assert.ok(offlineState.agent.goals.every(goal=>!goal.completed));
  await page.getByRole('button',{name:'Inspect saved run'}).click();
  await page.waitForFunction(id=>document.querySelector('#agent-select').value===id,outcome.agent_id);
  await page.locator('.buffer[data-typing-state=complete]').waitFor();
  assert.ok((await page.locator('.buffer').innerText()).includes('Saved reasoning cycles / 3'));
  await page.setViewportSize({width:390,height:844});await page.getByText('Run an offline autonomy workflow',{exact:true}).click();
  await page.screenshot({path:resolve(evidence,'offline-workflow-mobile.png')});
  // Explicit replay probe of a completed test request: no extra identity, cycle or charge.
  const replay=await fetch(site+'/api/autonomy',{method:'POST',headers:{Authorization:basic(operator),Origin:site,'Content-Type':'application/json','X-Supa-Action':'offline-autonomy'},body:JSON.stringify({request_id:outcome.agent_id,goal:workflowGoal,authorize_offline:true})});assert.equal(replay.status,409);
  assert.equal((await request(offlineBase+'/financials')).balance_microusd,6700);
  const screenshots=[];
  for(const [name,width,height]of [['desktop',1440,1000],['tablet',768,1024],['mobile',390,844]]){
    await page.setViewportSize({width,height});
    for(const [section,text]of [['intelligence','Verify persistent integration'],['memory','Hypothesis'],['economy','Simulated ledger'],['social','never published']]){
      await page.goto(site+'/site/#'+section,{waitUntil:'domcontentloaded'});await page.locator('html[data-ready=true]').waitFor();
      await page.waitForFunction(id=>[...document.querySelector('#agent-select').options].some(option=>option.value===id),agent);
      await page.locator('#agent-select').evaluate((el,id)=>{el.value=id;el.dispatchEvent(new Event('change'));},agent);
      await page.waitForFunction(()=>document.querySelector('.buffer').dataset.typingState==='complete');
      assert.ok((await page.locator('.buffer').innerText()).includes(text),`${name} ${section} saved data absent`);
      assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
      if(section==='intelligence'||(name==='desktop'&&section==='economy')){const file=`${name}-${section}.png`;await page.screenshot({path:resolve(evidence,file)});screenshots.push(file);}
    }
    if(width===390){await page.locator('.tabline').click();assert.equal(await page.locator('.tabline').getAttribute('aria-expanded'),'true');await page.keyboard.press('Escape');assert.equal(await page.locator('.tabline').getAttribute('aria-expanded'),'false');}
  }
  // API process restart: database state remains paused and memory/cycles survive.
  service.kill('SIGTERM');await new Promise(r=>service.once('exit',r));
  const restarted=launch(resolve(root,'target/debug/supa-api'),[],root,{SUPA_BIND:`127.0.0.1:${apiPort}`});await ready(api+'/ready',restarted);
  assert.equal((await request(base)).agent.lifecycle,'paused');assert.equal((await request(base+'/cycles')).length,cycles.length);assert.equal((await request(base+'/memory')).length,memory.length);assert.deepEqual(await request(base+'/financials'),financials);
  assert.equal((await request(offlineBase)).agent.lifecycle,'paused');assert.equal((await request(offlineBase+'/cycles')).length,3);assert.equal((await request(offlineBase+'/financials')).balance_microusd,6700);
  assert.deepEqual(errors,[]);
  const report={passed:true,provider:'offline',database:'isolated_local_test',real_postgres:true,real_rust_api:true,real_next_host:true,api_restart_recovery:true,worker_default_paused:true,public_projection:true,private_reads_protected:true,scoped_operator_credentials:true,public_token_private_access_denied:true,cross_user_spoofing_denied:true,unrestricted_writes_blocked:true,cycles:cycles.length,memories:memory.length,tool_actions:actions.length,simulated_expenses:financials.inference_microusd+financials.operating_microusd,offline_workflow:{agent_id:outcome.agent_id,cycles:offlineCycles.length,actions:offlineActions.length,memories:offlineMemory.length,lifecycle:'paused',simulated_expenses_microusd:3300,remaining_simulated_microusd:6700,duplicate_rejected:true,restart_preserved:true},screenshots:['offline-workflow-desktop.png','offline-workflow-mobile.png',...screenshots],viewports:[1440,768,390],browser_errors:errors,paid_requests:0,transactions:0};
  await writeFile(resolve(evidence,'system-check.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({...report,evidence_directory:evidence}));
  if(process.env.SUPA_REA_CDP_PORT){await page.goto(site+'/site/#intelligence');console.log(JSON.stringify({rea_origin:site,cdp_endpoint:`http://127.0.0.1:${process.env.SUPA_REA_CDP_PORT}`}));await new Promise(r=>{process.once('SIGTERM',r);process.once('SIGINT',r);});}
}finally{
  await browser?.close();for(const child of children)if(child.exitCode===null)child.kill('SIGTERM');
}
