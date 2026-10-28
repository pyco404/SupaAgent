#!/usr/bin/env node
/** Real, isolated PostgreSQL/API/worker/Next.js/browser lifecycle demonstration. No live inference. */
import assert from 'node:assert/strict';
import {randomBytes,randomUUID} from 'node:crypto';
import {spawn} from 'node:child_process';
import {createServer} from 'node:net';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {chromium} from '../web/node_modules/playwright-core/index.mjs';
const root=resolve(import.meta.dirname,'..'),database=process.env.TEST_DATABASE_URL;
assert.ok(database,'dedicated TEST_DATABASE_URL required');const parsed=new URL(database);
assert.ok(['127.0.0.1','localhost','[::1]'].includes(parsed.hostname)&&parsed.pathname.endsWith('_test'),'dedicated local test database required');
const evidence=resolve(process.env.SUPA_SCHEDULING_EVIDENCE_DIR??'artifacts/continuous-autonomy');await mkdir(evidence,{recursive:true});
async function port(){const s=createServer();await new Promise(r=>s.listen(0,'127.0.0.1',r));const p=s.address().port;await new Promise(r=>s.close(r));return p;}
const apiPort=await port(),sitePort=await port(),api=`http://127.0.0.1:${apiPort}`,site=`http://127.0.0.1:${sitePort}`;
const token=randomBytes(32).toString('hex'),operator={username:'isolated-operator',password:randomBytes(24).toString('hex')},visitor={username:'isolated-viewer',password:randomBytes(24).toString('hex')};
const basic=p=>'Basic '+Buffer.from(`${p.username}:${p.password}`).toString('base64');
const env={PATH:process.env.PATH,HOME:process.env.HOME,NODE_ENV:'production',NEXT_TELEMETRY_DISABLED:'1',DATABASE_URL:database,SUPA_API_TOKEN:token,SUPA_PROVIDER:'offline',SUPA_OPERATOR_ID:randomUUID(),SUPA_ALLOW_OPENAI:'false',SUPA_API_SCHEDULER_ENABLED:'false',SUPA_WALLET_ENABLED:'false',SUPA_SITE_ENV_FILE:'',SUPA_SITE_MODE:'staging',SUPA_SITE_ORIGIN:site,SUPA_BACKEND_URL:api,SUPA_WEB_USERNAME:operator.username,SUPA_WEB_PASSWORD:operator.password,SUPA_STAGING_USERNAME:visitor.username,SUPA_STAGING_PASSWORD:visitor.password};
const children=[];let browser;
function launch(cmd,args,cwd,extra={}){const child=spawn(cmd,args,{cwd,env:{...env,...extra},stdio:'ignore'});children.push(child);return child;}
async function ready(url,child){for(let n=0;n<120;n++){assert.equal(child.exitCode,null,'service stopped');try{if((await fetch(url,{signal:AbortSignal.timeout(500)})).ok)return;}catch{}await new Promise(r=>setTimeout(r,150));}throw Error('readiness timeout');}
async function halt(child){const exited=new Promise(r=>child.once('exit',r));child.kill('SIGTERM');await exited;}
async function request(path,body){const r=await fetch(api+path,{headers:{Authorization:`Bearer ${token}`,'Content-Type':'application/json'},...(body===undefined?{}:{method:'POST',body:JSON.stringify(body)})});assert.ok(r.ok,`HTTP ${r.status} at ${path}`);return r.json();}
async function waitFor(check){for(let n=0;n<300;n++){if(await check())return;await new Promise(r=>setTimeout(r,100));}throw Error('expected saved state did not arrive');}
try{
  let service=launch(resolve(root,'target/debug/supa-api'),[],root,{SUPA_BIND:`127.0.0.1:${apiPort}`});await ready(api+'/ready',service);
  const web=launch(process.execPath,['node_modules/next/dist/bin/next','start','--hostname','127.0.0.1','--port',String(sitePort)],resolve(root,'web'));await ready(site+'/healthz',web);
  const agent=await request('/agents',{name:'Continuous offline verification',capabilities:['read_memory','remember','evaluate_goal']});const id=agent.id,base=`/agents/${id}`;
  assert.equal(agent.config.scheduler_enabled,false);
  const approved=await request(base+'/goals',{description:'Review verified outcomes and retain useful lessons'}),unapproved=await request(base+'/goals',{description:'Unapproved fixture goal; scheduler must leave this untouched'});
  await request(base+'/simulated-funding',{kind:'seed',amount_microusd:10000,idempotency_key:randomUUID(),simulated:true});
  assert.equal((await fetch(site+'/api'+base+'/schedule',{headers:{Authorization:basic(visitor)}})).status,401);
  browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,...(process.env.SUPA_REA_CDP_PORT?{args:[`--remote-debugging-port=${Number(process.env.SUPA_REA_CDP_PORT)}`,'--remote-debugging-address=127.0.0.1']}:{})});
  const context=await browser.newContext({httpCredentials:operator,viewport:{width:1440,height:1000},reducedMotion:'reduce',colorScheme:'dark'}),page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
  async function controls(operation){
    await page.goto(site+'/site/#intelligence',{waitUntil:'domcontentloaded'});await page.waitForFunction(id=>[...document.querySelector('#agent-select').options].some(o=>o.value===id),id);
    await page.locator('#agent-select').evaluate((el,id)=>{el.value=id;el.dispatchEvent(new Event('change'));},id);
    await page.locator('.buffer[data-typing-state=complete]').waitFor();const panel=page.locator('.schedule-panel');await panel.locator('summary').click();await panel.getByLabel('Lifecycle operation').selectOption(operation);await panel.getByLabel('Operator reason').fill(`Explicit isolated ${operation} verification`);return panel;
  }
  async function submit(panel){await panel.locator('[name=schedule-authorization]').check();const response=page.waitForResponse(r=>r.url()===site+'/api'+base+'/schedule'&&r.request().method()==='POST');await panel.getByRole('button',{name:'Apply authorized lifecycle operation'}).click();const r=await response;assert.ok(r.ok(),`control HTTP ${r.status()}`);return {body:r.request().postDataJSON(),result:await r.json()};}
  let panel=await controls('start');await panel.locator('[name=approved-goal]').filter({visible:true}).first().check();
  await panel.locator('[name=interval_seconds]').fill('5');await panel.locator('[name=max_cycles]').fill('3');await panel.locator('[name=simulated_budget_microusd]').fill('3300');
  await page.screenshot({path:resolve(evidence,'desktop-authorization.png')});const started=await submit(panel);assert.deepEqual(started.body.policy.goal_ids,[approved.id]);assert.equal(started.result.lifecycle,'active');
  await new Promise(r=>setTimeout(r,1100));assert.equal((await request(base+'/cycles')).length,0); // API scheduling defaults off.
  async function worker(){const p=await port(),child=launch(resolve(root,'target/debug/supa-runtime'),[],root,{SUPA_WORKER_BIND:`127.0.0.1:${p}`,SUPA_WORKER_ENABLED:'true',SUPA_WORKER_POLL_SECONDS:'1'});await ready(`http://127.0.0.1:${p}/ready`,child);return child;}
  let workerProcess=await worker();await waitFor(async()=>(await request(base+'/cycles')).some(c=>c.status==='completed'));
  panel=await controls('pause');await submit(panel);const paused=await request(base+'/schedule'),beforeCycles=await request(base+'/cycles'),beforeFinancials=await request(base+'/financials');assert.equal(paused.lifecycle,'paused');assert.equal(paused.cycles_started,1);
  // An old start command is a read of current truth, not a new activation.
  assert.equal((await request(base+'/schedule',started.body)).lifecycle,'paused');
  await halt(workerProcess);await halt(service);service=launch(resolve(root,'target/debug/supa-api'),[],root,{SUPA_BIND:`127.0.0.1:${apiPort}`});await ready(api+'/ready',service);workerProcess=await worker();
  await new Promise(r=>setTimeout(r,1200));assert.deepEqual(await request(base+'/cycles'),beforeCycles);assert.deepEqual(await request(base+'/financials'),beforeFinancials);assert.equal((await request(base+'/schedule')).lifecycle,'paused');
  await page.setViewportSize({width:390,height:844});panel=await controls('resume');await page.screenshot({path:resolve(evidence,'mobile-paused.png')});const resumed=await submit(panel);assert.equal(resumed.result.cycles_started,1);assert.equal(resumed.result.reserved_microusd,1100);
  await waitFor(async()=>{const s=await request(base+'/schedule');return s.lifecycle==='paused'&&s.cycles_started===3;});
  const finished=await request(base+'/schedule'),cycles=await request(base+'/cycles'),actions=await request(base+'/actions'),memory=await request(base+'/memory'),financials=await request(base+'/financials'),goals=await request(base+'/goals'),inference=await request(base+'/inference');
  assert.equal(cycles.length,3);assert.ok(cycles.every(c=>c.status==='completed'));assert.ok(actions.every(a=>a.status==='succeeded'&&a.authorization_outcome==='allowed'));assert.equal(financials.balance_microusd,6700);assert.equal(finished.reserved_microusd,3300);assert.ok(inference.every(call=>call.policy.live===false));assert.equal(goals.find(g=>g.id===unapproved.id).suggested_progress_bps,0);
  panel=await controls('stop');await submit(panel);await halt(workerProcess);
  await page.setViewportSize({width:1440,height:1000});await page.goto(site+'/site/#intelligence');await page.locator('.buffer[data-typing-state=complete]').waitFor();await page.locator('.schedule-panel summary').click();await page.screenshot({path:resolve(evidence,'desktop-terminated.png')});
  const stopped=await request(base+'/schedule');assert.equal(stopped.lifecycle,'terminated');assert.equal(stopped.enabled,false);assert.deepEqual(await request(base+'/financials'),financials);
  assert.deepEqual(errors,[]);assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  const report={passed:true,real_postgres:true,real_rust_api:true,real_worker_process:true,real_next_host:true,provider:'offline',agent_id:id,approved_goal_ids:[approved.id],unapproved_goal_untouched:true,cycles:cycles.length,actions:actions.length,memories:memory.length,inference_calls:inference.length,live_inference_calls:0,simulated_seed_microusd:10000,simulated_expenses_microusd:3300,remaining_simulated_microusd:6700,lifecycle:stopped.lifecycle,api_and_worker_restart_preserved_pause:true,start_replay_did_not_reactivate:true,resume_preserved_limits:true,automatic_limit_pause:true,default_scheduler_disabled:true,public_controls_denied:true,transactions:0,browser_errors:errors,viewports:[1440,390],screenshots:['desktop-authorization.png','mobile-paused.png','desktop-terminated.png']};
  await writeFile(resolve(evidence,'system-check.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(report));
  if(process.env.SUPA_REA_CDP_PORT){console.log(JSON.stringify({rea_origin:site,cdp_endpoint:`http://127.0.0.1:${process.env.SUPA_REA_CDP_PORT}`}));await new Promise(r=>{process.once('SIGTERM',r);process.once('SIGINT',r);});}
}finally{await browser?.close();for(const child of children)if(child.exitCode===null)child.kill('SIGTERM');}
