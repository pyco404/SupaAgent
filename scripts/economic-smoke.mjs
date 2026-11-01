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
const apiBinary=process.env.SUPA_TEST_API_BINARY?resolve(process.env.SUPA_TEST_API_BINARY):resolve(root,'target/debug/supa-api');
const children=[],errors=[];let browser,agent;
function launch(command,args,cwd,extra={}){const child=spawn(command,args,{cwd,env:{...env,...extra},stdio:['ignore','ignore','ignore']});children.push(child);return child;}
async function ready(url,child){for(let i=0;i<120;i++){assert.equal(child.exitCode,null,'service exited');try{if((await fetch(url,{signal:AbortSignal.timeout(500)})).ok)return;}catch{}await new Promise(r=>setTimeout(r,150));}throw Error('service readiness timeout');}
async function request(path,body){const response=await fetch(api+path,{headers:{Authorization:`Bearer ${ownerToken}`,'Content-Type':'application/json'},...(body===undefined?{}:{method:'POST',body:JSON.stringify(body)})});assert.ok(response.ok,`Rust request failed: ${path} HTTP ${response.status}`);return response.json();}
async function frontend(path,person=operator){return fetch(site+path,{headers:person?{Authorization:basic(person)}:{},redirect:'manual'});}
try{
 const service=launch(apiBinary,[],root,{SUPA_BIND:`127.0.0.1:${apiPort}`});await ready(api+'/ready',service);
 const app=launch(process.execPath,['node_modules/next/dist/bin/next','start','--hostname','127.0.0.1','--port',String(webPort)],resolve(root,'web'));await ready(site+'/healthz',app);
 agent=(await request('/agents',{name:'Economic browser validation fixture',capabilities:['read_memory','evaluate_goal'],config:{scheduler_enabled:false,interval_seconds:60}})).id;
 const base=`/agents/${agent}`;
 await request(base+'/lifecycle',{state:'active',reason:'Explicit disposable economic validation'});
 const serviceId=randomUUID();
 await request(base+'/services',{id:serviceId,authorize:true,spec:{name:'Supplied text summary',kind:'summary',price_atoms:5000,estimated_inference_atoms:1000,estimated_infrastructure_atoms:100,max_input_bytes:8000,max_attempts:2}});
 await request(base+'/economy/capital',{request_id:randomUUID(),amount_atoms:10000,authorize_simulated:true});
 await request(base+'/economy/policy',{ceiling_atoms:5000,reserve_atoms:1000,destinations:['inference','infrastructure'],authorize_simulated:true});
 browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,...(process.env.SUPA_REA_CDP_PORT?{args:[`--remote-debugging-port=${Number(process.env.SUPA_REA_CDP_PORT)}`,'--remote-debugging-address=127.0.0.1']}:{})});
 const context=await browser.newContext({httpCredentials:operator,reducedMotion:'reduce',colorScheme:'dark'}),page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));
 async function openEconomy(width=1440,height=1000){await page.setViewportSize({width,height});await page.goto(site+'/economy');await page.locator('html[data-ready=true]').waitFor();await page.waitForFunction(id=>[...document.querySelector('#agent-select').options].some(o=>o.value===id),agent);await page.locator('#agent-select').evaluate((el,id)=>{el.value=id;el.dispatchEvent(new Event('change'));},agent);await page.locator('.buffer[data-typing-state=complete]').waitFor();await page.locator('[data-economy=jobs]>summary').click();}
 await openEconomy();await page.getByLabel('Supplied input / no secrets').fill(JSON.stringify({objective:'Summarize supplied fact',material:'A supplied fact for a bounded summary. A second supplied fact.',requirements:['supplied fact']}));await page.locator('[data-economy=jobs]>form input[type=checkbox]').check();
 const created=page.waitForResponse(r=>r.url()===site+'/api/jobs'&&r.request().method()==='POST');await page.getByRole('button',{name:'Create simulated job',exact:true}).click();const creation=await created;assert.equal(creation.status(),200);let job=await creation.json();
 for(const [state,button]of [['created','request payment'],['awaiting_payment','Record simulated payment'],['funded','accept'],['accepted','execute']]){
  assert.equal(job.state,state);await openEconomy();const block=page.locator(`[data-job-id="${job.id}"]`);await block.locator(':scope>summary').click();await block.getByRole('checkbox').check();const done=page.waitForResponse(r=>r.url().includes('/api/jobs/'+job.id+'/')&&r.request().method()==='POST');await block.getByRole('button',{name:button,exact:true}).click();const response=await done;assert.equal(response.status(),200,await response.text());job=await response.json();
 }
 assert.equal(job.state,'completed');assert.equal(job.output.analysis.sentences[0],'A supplied fact for a bounded summary');assert.equal(job.earned_atoms,5000);assert.equal(job.output.work.status,'completed');assert.equal(job.output.work.steps.length,3);assert.equal(job.output.work.evidence[0].source,'supplied_material');assert.equal(job.output.work.provider_requests,1);
 const accounts=await request(base+'/economy');assert.equal(accounts.accounts.networks[0].liability_atoms,0);assert.equal(accounts.accounts.networks[0].net_result_atoms,3900);assert.equal(accounts.accounts.networks[1].earned_atoms,0);
 const screenshots=[];
 for(const [name,width,height]of [['desktop',1440,1000],['tablet',768,1024],['mobile',390,844]]){await openEconomy(width,height);await page.locator(`[data-job-id="${job.id}"]>summary`).click();assert.ok(await page.locator('[data-economy=jobs]').innerText().then(t=>t.includes('Output SHA-256')&&t.includes('Bounded offline work / completed')&&t.includes('unverified statement')));assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));const file=`economic-${name}.png`;await page.screenshot({path:resolve(evidence,file)});screenshots.push(file);}
 for(const section of ['home','agents','intelligence','memory','economy','social','trading','token-launch','factory','about']){await page.goto(site+(section==='home'?'/':'/'+section));if(section==='home'){await page.locator('html[data-ready=true]').waitFor();await page.locator('a.enter').click();}await page.locator('.buffer[data-typing-state=complete]').waitFor();assert.ok((await page.locator('.buffer').innerText()).length>20);}
 service.kill('SIGTERM');await new Promise(r=>service.once('exit',r));const restarted=launch(apiBinary,[],root,{SUPA_BIND:`127.0.0.1:${apiPort}`});await ready(api+'/ready',restarted);assert.equal((await request('/jobs/'+job.id)).job.output_sha256,job.output_sha256);
 await openEconomy(390,844);await page.locator(`[data-job-id="${job.id}"]>summary`).click();assert.deepEqual(errors,[]);
 const report={passed:true,database:'disposable_postgresql',bounded_work_contract:'bounded_work_v1',grounded_report:true,provider_requests:job.output.work.provider_requests,rust_api:true,next_frontend:true,workflow:['created','awaiting_payment','funded','accepted','executing','completed'],simulated_earned_atoms:5000,simulated_expenses_atoms:1100,simulated_net_result_atoms:3900,devnet_earned_atoms:0,real_earned_usdc:0,paid_provider_requests:0,transfers:0,all_ten_sections:true,api_restart_preserved:true,browser_errors:errors,screenshots};await writeFile(resolve(evidence,'economic-system-check.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({...report,evidence_directory:evidence}));
 if(process.env.SUPA_REA_CDP_PORT){console.log(JSON.stringify({rea_origin:site,cdp_endpoint:`http://127.0.0.1:${process.env.SUPA_REA_CDP_PORT}`}));await new Promise(r=>{process.once('SIGTERM',r);process.once('SIGINT',r);});}
}finally{await browser?.close();for(const child of children)if(child.exitCode===null)child.kill('SIGTERM');}
