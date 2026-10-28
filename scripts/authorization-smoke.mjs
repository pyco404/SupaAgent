#!/usr/bin/env node
/** Isolated real API/PostgreSQL/browser verification. Worker enabled only with all agents paused. */
import assert from 'node:assert/strict';
import {randomBytes,randomUUID} from 'node:crypto';
import {spawn,execFileSync} from 'node:child_process';
import {createServer} from 'node:net';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {chromium} from '../web/node_modules/playwright-core/index.mjs';
const root=resolve(import.meta.dirname,'..'),database=process.env.TEST_DATABASE_URL;
assert.ok(database,'dedicated test database required');const db=new URL(database);
assert.ok(['127.0.0.1','localhost','[::1]'].includes(db.hostname)&&db.pathname.endsWith('_test')&&!db.search,'dedicated local test database required');
const schema='authorization_browser_'+randomUUID().replaceAll('-','');
// Connection settings stay in the environment, never command arguments or logs.
execFileSync('psql',['-v','ON_ERROR_STOP=1','-c',`CREATE SCHEMA ${schema}`],{env:{PATH:process.env.PATH,PGHOST:db.hostname,PGPORT:db.port||'5432',PGUSER:decodeURIComponent(db.username),PGPASSWORD:decodeURIComponent(db.password),PGDATABASE:decodeURIComponent(db.pathname.slice(1))},stdio:'ignore'});
const scoped=new URL(database);scoped.searchParams.set('options',`-csearch_path=${schema}`);
const evidence=resolve(process.env.SUPA_AUTHORIZATION_EVIDENCE_DIR??'artifacts/authorization-renewal');await mkdir(evidence,{recursive:true});
async function port(){const s=createServer();await new Promise(r=>s.listen(0,'127.0.0.1',r));const p=s.address().port;await new Promise(r=>s.close(r));return p;}
const apiPort=await port(),sitePort=await port(),api=`http://127.0.0.1:${apiPort}`,site=`http://127.0.0.1:${sitePort}`;
const account=role=>({id:randomUUID(),role,token:randomBytes(32).toString('hex'),username:role,password:randomBytes(24).toString('hex')});
const owner=account('owner'),viewer=account('viewer'),other=account('owner');other.username='other';
const basic=p=>'Basic '+Buffer.from(`${p.username}:${p.password}`).toString('base64');
const env={PATH:process.env.PATH,HOME:process.env.HOME,DATABASE_URL:scoped.href,SUPA_API_TOKEN:owner.token,SUPA_OPERATOR_ID:owner.id,SUPA_OPERATOR_CREDENTIALS:JSON.stringify([viewer,other].map(({id,role,token})=>({id,role,token}))),SUPA_PROVIDER:'offline',SUPA_API_SCHEDULER_ENABLED:'false',SUPA_WALLET_ENABLED:'false',SUPA_SITE_ENV_FILE:'',SUPA_BACKEND_URL:api,SUPA_WEB_USERNAME:owner.username,SUPA_WEB_PASSWORD:owner.password,SUPA_WEB_OPERATORS:JSON.stringify([viewer,other].map(({username,password,token})=>({username,password,api_token:token})))};
const children=[];let browser;const contexts=[];
function launch(cmd,args,cwd,extra={}){const child=spawn(cmd,args,{cwd,env:{...env,...extra},stdio:'ignore'});children.push(child);return child;}
async function ready(url,child){for(let i=0;i<150;i++){assert.equal(child.exitCode,null,'service stopped');try{if((await fetch(url,{signal:AbortSignal.timeout(500)})).ok)return;}catch{}await new Promise(r=>setTimeout(r,100));}throw Error('readiness timeout');}
async function halt(child){const exited=new Promise(r=>child.once('exit',r));child.kill('SIGTERM');await exited;}
async function request(path,body,user=owner){const r=await fetch(api+path,{headers:{Authorization:`Bearer ${user.token}`,'Content-Type':'application/json'},...(body===undefined?{}:{method:'POST',body:JSON.stringify(body)})});return {status:r.status,body:await r.json()};}
async function ok(path,body,user=owner){const result=await request(path,body,user);assert.equal(result.status,200,`${path}: HTTP ${result.status}`);return result.body;}
try{
 let service=launch(resolve(root,'target/debug/supa-api'),[],root,{SUPA_BIND:`127.0.0.1:${apiPort}`});await ready(api+'/ready',service);
 const web=launch(process.execPath,['server.mjs'],resolve(root,'reference-web'),{PORT:String(sitePort)});await ready(site+'/healthz',web);
 const agent=await ok('/agents',{name:'Isolated authorization verification',capabilities:['read_memory','remember','evaluate_goal']}),id=agent.id,base=`/agents/${id}`;
 const goal=await ok(base+'/goals',{description:'Review verified outcomes after explicit finite approval'});
 await ok(base+'/access',{principal_id:viewer.id,role:'viewer'});
 assert.equal((await request(base+'/memory',undefined,other)).status,403);
 assert.equal((await request(base+'/memory',undefined,viewer)).status,200);
 const initialFinancials=await ok(base+'/financials');
 browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,...(process.env.SUPA_REA_CDP_PORT?{args:[`--remote-debugging-port=${Number(process.env.SUPA_REA_CDP_PORT)}`,'--remote-debugging-address=127.0.0.1']}:{})});
 const context=await browser.newContext({httpCredentials:owner,viewport:{width:1440,height:1000},reducedMotion:'reduce',colorScheme:'dark'});contexts.push(context);
 const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
 async function inspect(userPage=page){await userPage.goto(site+'/operator',{waitUntil:'domcontentloaded'});await userPage.goto(site+'/site/#intelligence',{waitUntil:'domcontentloaded'});await userPage.waitForFunction(id=>[...document.querySelector('#agent-select').options].some(o=>o.value===id),id);await userPage.locator('#agent-select').evaluate((el,id)=>{el.value=id;el.dispatchEvent(new Event('change'));},id);await userPage.locator('.buffer[data-typing-state=complete]').waitFor();await userPage.locator('.schedule-panel summary').click();return userPage.locator('.schedule-panel');}
 async function operation(name){const panel=await inspect();await panel.getByLabel('Lifecycle operation').selectOption(name);if(['start','renew'].includes(name)){await panel.locator('[name=approved-goal]').check();await panel.locator('[name=max_cycles]').fill('3');await panel.locator('[name=simulated_budget_microusd]').fill('3300');}await panel.getByLabel('Operator reason').fill(`Explicit isolated ${name}; scheduler disabled`);return panel;}
 async function submit(panel){await panel.locator('[name=schedule-authorization]').check();const response=page.waitForResponse(r=>r.url()===site+'/api'+base+'/schedule'&&r.request().method()==='POST');await panel.getByRole('button',{name:'Apply authorized lifecycle operation'}).click();const r=await response;assert.equal(r.status(),200);return r.json();}
 let panel=await operation('start');await submit(panel);panel=await operation('pause');await submit(panel);
 const first=await ok(base+'/authorizations');assert.equal(first.length,1);
 panel=await operation('renew');await panel.locator('[name=schedule-authorization]').check();await panel.locator('[name=max_cycles]').fill('2');assert.equal(await panel.locator('[name=schedule-authorization]').isChecked(),false);
 await submit(panel);panel=await operation('pause');await submit(panel);
 const history=await ok(base+'/authorizations');assert.equal(history.length,2);assert.equal(history.filter(a=>a.closed_at===null).length,1);assert.ok(history.every(a=>a.approved_by===owner.id));
 assert.deepEqual(await ok(base+'/financials'),initialFinancials);assert.deepEqual(await ok(base+'/cycles'),[]);
 const denied=await request(base+'/schedule',{request_id:randomUUID(),expected_revision:(await ok(base+'/schedule')).revision,operation:'renew',authorize_offline:true,reason:'Viewer escalation must fail',policy:{goal_ids:[goal.id],interval_seconds:5,max_cycles:1,simulated_budget_microusd:1100,valid_for_seconds:60}},viewer);assert.equal(denied.status,403);
 await halt(service);service=launch(resolve(root,'target/debug/supa-api'),[],root,{SUPA_BIND:`127.0.0.1:${apiPort}`});await ready(api+'/ready',service);assert.equal((await ok(base+'/schedule')).lifecycle,'paused');assert.deepEqual(await ok(base+'/authorizations'),history);
 // Real heartbeat/tick health with zero execution: all scoped agents are paused.
 const workerPort=await port(),worker=launch(resolve(root,'target/debug/supa-runtime'),[],root,{SUPA_WORKER_ENABLED:'true',SUPA_WORKER_POLL_SECONDS:'1',SUPA_WORKER_BIND:`127.0.0.1:${workerPort}`});await ready(`http://127.0.0.1:${workerPort}/ready`,worker);
 for(let n=0;n<50;n++){if((await ok(base+'/execution-health')).status==='healthy')break;await new Promise(r=>setTimeout(r,100));}
 const health=await ok(base+'/execution-health');assert.equal(health.status,'healthy');assert.equal(health.execution_running,false);
 panel=await operation('renew');assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:resolve(evidence,'desktop-owner.png')});
 await page.setViewportSize({width:390,height:844});panel=await operation('renew');await page.screenshot({path:resolve(evidence,'mobile-owner.png')});
 const viewerContext=await browser.newContext({httpCredentials:viewer,viewport:{width:390,height:844},reducedMotion:'reduce',colorScheme:'dark'});contexts.push(viewerContext);const viewerPage=await viewerContext.newPage();const viewerPanel=await inspect(viewerPage);assert.equal(await viewerPanel.locator('form').isVisible(),false);await viewerPage.screenshot({path:resolve(evidence,'mobile-viewer.png')});
 assert.ok(await viewerPage.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));assert.deepEqual(errors,[]);
 const report={passed:true,real_postgres:true,real_rust_api:true,real_frontend_adapter:true,real_worker_heartbeat:true,worker_health:health.status,verified_running_execution:false,agents_paused:true,planning_cycles:0,paid_requests:0,financial_records_changed:false,renewal_epochs:history.length,approver_recorded:true,api_restart_preserved_authorizations:true,cross_user_denied:true,viewer_control_denied:true,fresh_consent_after_limit_change:true,browser_errors:errors,viewports:[1440,390],screenshots:['desktop-owner.png','mobile-owner.png','mobile-viewer.png']};
 await writeFile(resolve(evidence,'system-check.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(report));
 if(process.env.SUPA_REA_CDP_PORT){console.log(JSON.stringify({rea_origin:site,cdp_endpoint:`http://127.0.0.1:${process.env.SUPA_REA_CDP_PORT}`,owner_target_url:page.url()}));await new Promise(r=>{process.once('SIGTERM',r);process.once('SIGINT',r);});}
 await halt(worker);
}finally{await browser?.close();for(const child of children)if(child.exitCode===null)child.kill('SIGTERM');}
