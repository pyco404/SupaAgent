#!/usr/bin/env node
/** Actual Next -> Rust -> PostgreSQL, synthetic provider only. Never constructs an OpenAI client. */
import assert from 'node:assert/strict';
import {randomBytes,randomUUID} from 'node:crypto';
import {spawn} from 'node:child_process';
import {createServer} from 'node:net';
import {mkdir,mkdtemp,writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve} from 'node:path';
import {chromium} from '../web/node_modules/playwright-core/index.mjs';
const root=resolve(import.meta.dirname,'..'),database=process.env.TEST_DATABASE_URL;assert.ok(database);const parsed=new URL(database);assert.equal(parsed.hostname,'localhost');assert.equal(parsed.port,'55680');assert.equal(parsed.pathname,'/supa_m10_test');
const binary=resolve(process.env.SUPA_TEST_WORK_FIXTURE??'/tmp/supa-m9-target/debug/examples/work_fixture');
const evidence=process.env.SUPA_WORK_EVIDENCE_DIR?resolve(process.env.SUPA_WORK_EVIDENCE_DIR):await mkdtemp(resolve(tmpdir(),'supa-m10-browser-'));await mkdir(evidence,{recursive:true});
async function port(){const server=createServer();await new Promise(r=>server.listen(0,'127.0.0.1',r));const value=server.address().port;await new Promise(r=>server.close(r));return value;}
const apiPort=await port(),webPort=await port(),api=`http://127.0.0.1:${apiPort}`,site=`http://127.0.0.1:${webPort}`;
const ownerId=randomUUID(),ownerToken=randomBytes(32).toString('hex'),serviceToken=randomBytes(32).toString('hex'),operator={username:'work-fixture',password:randomBytes(24).toString('hex')};
const env={PATH:process.env.PATH,HOME:process.env.HOME,NODE_ENV:'production',__NEXT_PROCESSED_ENV:'true',NEXT_TELEMETRY_DISABLED:'1',TEST_DATABASE_URL:database,SUPA_TEST_OWNER_ID:ownerId,SUPA_TEST_OWNER_TOKEN:ownerToken,SUPA_TEST_SERVICE_TOKEN:serviceToken,SUPA_TEST_BIND:`127.0.0.1:${apiPort}`,SUPA_BACKEND_URL:api,SUPA_API_TOKEN:serviceToken,SUPA_SITE_MODE:'staging',SUPA_SITE_ENV_FILE:'',SUPA_SITE_ORIGIN:site,SUPA_PRIVATE_STAGING:'true',SUPA_WEB_OPERATORS:JSON.stringify([{...operator,api_token:ownerToken}]),SUPA_STAGING_USERNAME:'work-fixture-visitor',SUPA_STAGING_PASSWORD:randomBytes(24).toString('hex'),SUPA_ALLOW_OPENAI:'false',SUPA_CONTROLLED_INFERENCE:'false',SUPA_CONTROLLED_WORK:'false',SUPA_API_SCHEDULER_ENABLED:'false'};
const children=[];let browser;
function launch(command,args,cwd){const child=spawn(command,args,{cwd,env,stdio:['ignore','pipe','pipe']});const lines=[];child.stdout.on('data',data=>lines.push(data.toString()));child.stderr.on('data',data=>lines.push(data.toString()));child.on('exit',()=>void writeFile(resolve(evidence,command===binary?'fixture-process.log':'frontend-process.log'),lines.join('')));children.push(child);return child;}
async function ready(url,child,headers={}){for(let i=0;i<160;i++){assert.equal(child.exitCode,null,'fixture exited');try{if((await fetch(url,{headers,signal:AbortSignal.timeout(500)})).ok)return;}catch{}await new Promise(r=>setTimeout(r,100));}throw Error('local readiness timeout');}
async function get(path){const r=await fetch(api+path,{headers:{Authorization:`Bearer ${ownerToken}`}});assert.equal(r.status,200);return r.json();}
try{
 const service=launch(binary,[],root);await ready(api+'/health',service,{Authorization:`Bearer ${serviceToken}`});
 const frontend=launch(process.execPath,['node_modules/next/dist/bin/next','start','--hostname','127.0.0.1','--port',String(webPort)],resolve(root,'web'));await ready(site+'/healthz',frontend);
 const agents=await get('/agents');const agent=agents.find(a=>a.name==='Disposable mocked GPT work'&&a.lifecycle==='paused');assert.ok(agent);
 const metadata=await get(`/agents/${agent.id}/live-work`);assert.equal(metadata.mode,'live_work_evaluation');assert.equal(metadata.policy.model,'mock-work');assert.equal(metadata.continuous,false);
 browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,...(process.env.SUPA_REA_CDP_PORT?{args:[`--remote-debugging-port=${Number(process.env.SUPA_REA_CDP_PORT)}`,'--remote-debugging-address=127.0.0.1']}:{})});
 const context=await browser.newContext({httpCredentials:operator,colorScheme:'dark',reducedMotion:'reduce'}),page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
 async function open(section,width=1440,height=1000){await page.setViewportSize({width,height});await page.goto(site+'/'+section);await page.locator('html[data-ready=true]').waitFor();await page.waitForFunction(id=>[...document.querySelector('#agent-select').options].some(o=>o.value===id),agent.id);await page.locator('#agent-select').evaluate((el,id)=>{el.value=id;el.dispatchEvent(new Event('change'));},agent.id);await page.locator('[data-live-work=evaluation]>summary').waitFor();await page.locator('.buffer[data-typing-state=complete]').waitFor();await page.locator('[data-live-work=evaluation]>summary').click();}
 for(const memory of [false,true]){
  await open('economy');const job=metadata.jobs.find(j=>memory?j.objective.startsWith('Review verified cycle'):j.objective.startsWith('Compare the supplied'));
  await page.getByLabel('Unpaid evaluation job',{exact:true}).selectOption(job.id);
  const form=page.locator('[data-live-work=evaluation] form');if(memory)await form.getByRole('checkbox').nth(0).check();await form.getByRole('checkbox').nth(1).check();
  const done=page.waitForResponse(r=>r.url()===site+`/api/agents/${agent.id}/live-work`&&r.request().method()==='POST');await page.getByRole('button',{name:'Authorize one GPT work evaluation',exact:true}).click();const response=await done;assert.equal(response.status(),200,await response.text());const result=await response.json();assert.equal(result.status,'awaiting_approval');assert.equal(result.work.provider_evidence,'mock');assert.equal(result.accounted_microusd,35);assert.equal(result.work.report.memory_ids.length,memory?1:0);
 }
 const results=await get(`/agents/${agent.id}/live-work`);assert.equal(results.sessions.length,2);assert.ok(results.sessions.every(s=>s.work.provider_evidence==='mock'));
 const economy=await get(`/agents/${agent.id}/economy`);assert.ok(economy.jobs.every(j=>j.state==='created'&&j.received_atoms===0&&j.earned_atoms===0));assert.ok(economy.accounts.networks.every(n=>n.earned_atoms===0&&n.cash_atoms===0&&n.inference_atoms===0));
 const screenshots=[];
 for(const [name,width,height]of [['desktop',1440,1000],['tablet',768,1024],['mobile',390,844]]){
  await open('intelligence',width,height);await page.locator('[data-work-session]>summary').first().click();const block=page.locator('[data-work-session]').first();assert.match(await block.innerText(),/synthetic provider usage/);assert.match(await block.innerText(),/Provider usage: 100 input/);assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));const file=`work-${name}.png`;await page.screenshot({path:resolve(evidence,file)});screenshots.push(file);
 }
 assert.deepEqual(errors,[]);const report={passed:true,provider_evidence:'mock',actual_openai_requests:0,actual_charges:0,mocked_usage_microusd:70,memory_free_references:0,memory_assisted_references:1,commercial_deliveries:0,customer_payments:0,economic_entries:0,agent_lifecycle:'paused',rust_api:true,postgresql:true,next_frontend:true,viewports:['desktop','tablet','mobile'],browser_errors:errors,screenshots};await writeFile(resolve(evidence,'work-system-check.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({...report,evidence_directory:evidence}));
 if(process.env.SUPA_REA_CDP_PORT){console.log(JSON.stringify({rea_origin:site,cdp_endpoint:`http://127.0.0.1:${process.env.SUPA_REA_CDP_PORT}`}));await new Promise(r=>{process.once('SIGTERM',r);process.once('SIGINT',r);});}
}finally{await browser?.close();for(const child of children)if(child.exitCode===null)child.kill('SIGTERM');}
