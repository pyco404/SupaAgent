#!/usr/bin/env node
/** Real PostgreSQL/API/frontend, explicitly mocked inference; never loads evaluation keys. */
import assert from 'node:assert/strict';
import {randomBytes,randomUUID} from 'node:crypto';
import {spawn,execFileSync} from 'node:child_process';
import {createServer} from 'node:net';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {chromium} from '../web/node_modules/playwright-core/index.mjs';
const root=resolve(import.meta.dirname,'..'),database=process.env.TEST_DATABASE_URL;assert.ok(database);
const db=new URL(database);assert.ok(['127.0.0.1','localhost','[::1]'].includes(db.hostname)&&db.pathname.endsWith('_test')&&!db.search);
const schema='controlled_browser_'+randomUUID().replaceAll('-','');
execFileSync('psql',['-v','ON_ERROR_STOP=1','-c',`CREATE SCHEMA ${schema}`],{env:{PATH:process.env.PATH,PGHOST:db.hostname,PGPORT:db.port||'5432',PGUSER:decodeURIComponent(db.username),PGPASSWORD:decodeURIComponent(db.password),PGDATABASE:decodeURIComponent(db.pathname.slice(1))},stdio:'ignore'});
const scoped=new URL(database);scoped.searchParams.set('options',`-csearch_path=${schema}`);
async function port(){const s=createServer();await new Promise(r=>s.listen(0,'127.0.0.1',r));const p=s.address().port;await new Promise(r=>s.close(r));return p;}
const apiPort=await port(),sitePort=await port(),site=`http://127.0.0.1:${sitePort}`,api=`http://127.0.0.1:${apiPort}`;
const token=randomBytes(32).toString('hex'),owner=randomUUID(),password=randomBytes(24).toString('hex');
const env={PATH:process.env.PATH,HOME:process.env.HOME,DATABASE_URL:scoped.href,SUPA_API_TOKEN:token,SUPA_OPERATOR_ID:owner,SUPA_BIND:`127.0.0.1:${apiPort}`,SUPA_SITE_ENV_FILE:'',SUPA_BACKEND_URL:api,SUPA_WEB_USERNAME:'fixture',SUPA_WEB_PASSWORD:password,PORT:String(sitePort)};
const evidence=resolve(process.env.SUPA_CONTROLLED_EVIDENCE_DIR??'artifacts/controlled-inference');await mkdir(evidence,{recursive:true});
const children=[];let browser;
function launch(cmd,args,cwd){const p=spawn(cmd,args,{cwd,env,stdio:'ignore'});children.push(p);return p;}
async function ready(url,p){for(let n=0;n<200;n++){assert.equal(p.exitCode,null,'fixture service stopped');try{if((await fetch(url,{signal:AbortSignal.timeout(500)})).ok)return;}catch{}await new Promise(r=>setTimeout(r,100));}throw Error('fixture readiness timeout');}
async function get(path){const r=await fetch(api+path,{headers:{Authorization:`Bearer ${token}`}});assert.equal(r.status,200);return r.json();}
try{
 const service=launch(resolve(root,'target/debug/examples/controlled_fixture'),['--authorize-isolated-fixture'],root);await ready(api+'/ready',service);
 const web=launch(process.execPath,['server.mjs'],resolve(root,'reference-web'));await ready(site+'/healthz',web);
 const agent=(await get('/agents'))[0],base=`/agents/${agent.id}`;const before=await get(base+'/financials');
 browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,...(process.env.SUPA_REA_CDP_PORT?{args:[`--remote-debugging-port=${Number(process.env.SUPA_REA_CDP_PORT)}`,'--remote-debugging-address=127.0.0.1']}:{})});
 const context=await browser.newContext({httpCredentials:{username:'fixture',password},viewport:{width:1440,height:1000},reducedMotion:'reduce',colorScheme:'dark'});const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
 async function open(){await page.goto(site+'/operator',{waitUntil:'domcontentloaded'});await page.goto(site+'/site/#intelligence',{waitUntil:'domcontentloaded'});await page.waitForFunction(id=>[...document.querySelector('#agent-select').options].some(o=>o.value===id),agent.id);await page.locator('#agent-select').evaluate((el,id)=>{el.value=id;el.dispatchEvent(new Event('change'));},agent.id);await page.locator('.buffer[data-typing-state=complete]').waitFor();await page.locator('.live-panel summary').first().click();return page.locator('.live-panel');}
 let panel=await open();assert.equal(await panel.locator('.live-form').isVisible(),true);await panel.locator('[name=live-goal]').fill('Review the supplied fixture hypothesis without inventing facts.');await panel.locator('[name=live-ceiling]').fill('10000');await panel.locator('[name=authorize-paid]').check();await panel.locator('[name=live-ceiling]').fill('9000');assert.equal(await panel.locator('[name=authorize-paid]').isChecked(),false);await panel.locator('[name=authorize-paid]').check();
 const response=page.waitForResponse(r=>r.url()===site+'/api'+base+'/live'&&r.request().method()==='POST');await panel.getByRole('button',{name:'Authorize one paid GPT request'}).click();const r=await response;assert.equal(r.status(),200);const session=await r.json();assert.equal(session.status,'awaiting_approval');assert.equal(session.generation_requests,1);assert.ok(session.proposal.summary.startsWith('MOCK PROVIDER'));assert.equal(session.tool_result,null);
 panel=await open();await panel.locator('details summary').click();await page.screenshot({path:resolve(evidence,'desktop-proposal.png')});
 const approved=page.waitForResponse(r=>r.url().endsWith('/approval')&&r.request().method()==='POST');await panel.getByRole('button',{name:'Approve proposed memory read'}).click();assert.equal((await approved).status(),200);
 await page.setViewportSize({width:390,height:844});panel=await open();await panel.locator('details summary').click();await panel.locator('details').scrollIntoViewIfNeeded();assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:resolve(evidence,'mobile-result.png')});
 const saved=(await get(base+'/live')).sessions[0];assert.equal(saved.status,'completed');assert.equal(saved.tool_result.executed_actions,1);assert.deepEqual(await get(base+'/financials'),before);assert.deepEqual(errors,[]);
 const report={passed:true,real_postgres:true,real_rust_api:true,real_frontend_adapter:true,provider:'mock',actual_openai_requests:0,paid_charges:0,mock_generation_attempts:saved.generation_requests,mock_usage:saved.usage,mock_accounted_microusd:saved.accounted_microusd,session_status:saved.status,tool_actions:saved.tool_result.executed_actions,simulated_ledger_unchanged:true,consent_cleared_after_limit_change:true,mobile_overflow:false,browser_errors:errors,viewports:[{width:1440,height:1000},{width:390,height:844}]};await writeFile(resolve(evidence,'system-check.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(report));
 if(process.env.SUPA_REA_CDP_PORT){console.log(JSON.stringify({rea_origin:site,cdp_endpoint:`http://127.0.0.1:${process.env.SUPA_REA_CDP_PORT}`,target:page.url()}));await new Promise(r=>{process.once('SIGTERM',r);process.once('SIGINT',r);});}
}finally{await browser?.close();for(const child of children)if(child.exitCode===null)child.kill('SIGTERM');}
