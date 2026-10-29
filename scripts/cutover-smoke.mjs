#!/usr/bin/env node
/** Isolated review only. Real candidate is never transitioned or used as the app backend. */
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir,lstat} from 'node:fs/promises';
import {parseEnv} from 'node:util';
import {resolve} from 'node:path';
import {randomBytes} from 'node:crypto';
import {spawn,spawnSync,execFileSync} from 'node:child_process';
import {createServer} from 'node:net';
import {chromium} from '../web/node_modules/playwright-core/index.mjs';
const root=resolve(import.meta.dirname,'..'),evidence=resolve(root,'artifacts/cutover-review');await mkdir(evidence,{recursive:true});
function fixture(action,path){return JSON.parse(execFileSync('python3',['scripts/cutover-fixture.py',action,...(path?[path]:[])],{cwd:root,encoding:'utf8'}));}
const prepared=fixture('prepare'),fixtureRoot=prepared.root,config=JSON.parse(await readFile(resolve(fixtureRoot,'fixture.json'),'utf8'));
async function port(){const s=createServer();await new Promise(r=>s.listen(0,'127.0.0.1',r));const p=s.address().port;await new Promise(r=>s.close(r));return p;}
const apiPort=await port(),sitePort=await port(),api=`http://127.0.0.1:${apiPort}`,site=`http://127.0.0.1:${sitePort}`;
const database=execFileSync('python3',['-c',`import importlib.util; s=importlib.util.spec_from_file_location('e','scripts/development-epoch.py'); m=importlib.util.module_from_spec(s); s.loader.exec_module(m); print(m.url('${config.database}'))`],{cwd:root,encoding:'utf8'}).trim();
const token=randomBytes(32).toString('hex');
const original={DATABASE_URL:database,SUPA_BIND:`127.0.0.1:${apiPort}`,SUPA_API_TOKEN:token,SUPA_PROVIDER:'offline',SUPA_ALLOW_OPENAI:'false',SUPA_CONTROLLED_INFERENCE:'false',SUPA_API_SCHEDULER_ENABLED:'false',SUPA_WALLET_ENABLED:'false',SUPA_PRIVATE_STAGING:'false',SUPA_EXPECTED_EPOCH_ID:config.epoch_id,SUPA_EPOCH_READONLY_PREVIEW:'true',SUPA_EPOCH_MODE:'preview',SUPA_OPERATOR_CREDENTIALS:'[]'};
const lines=v=>Object.entries(v).map(([k,v])=>k+'='+JSON.stringify(v)).join('\n')+'\n';
await writeFile(resolve(fixtureRoot,'api.env'),lines(original),{flag:'wx',mode:0o600});await writeFile(resolve(fixtureRoot,'site.env'),lines({SUPA_BACKEND_URL:api,SUPA_API_TOKEN:token}),{flag:'wx',mode:0o600});
const activated=fixture('activate',fixtureRoot);
const base={PATH:process.env.PATH,HOME:process.env.HOME},children=[];let browser,service,web;
async function apiEnv(){const p=resolve(fixtureRoot,'api.env');assert.equal((await lstat(p)).mode&0o077,0);return {...base,...parseEnv(await readFile(p,'utf8'))};}
function launch(cmd,args,cwd,env){const c=spawn(cmd,args,{cwd,env,stdio:'ignore'});children.push(c);return c;}
async function stop(child){if(child?.exitCode===null){child.kill('SIGTERM');await new Promise(r=>child.once('exit',r));}}
async function ready(url,c){for(let n=0;n<100;n++){assert.equal(c.exitCode,null);try{if((await fetch(url,{signal:AbortSignal.timeout(500)})).ok)return;}catch{}await new Promise(r=>setTimeout(r,100));}throw Error('fixture readiness timeout');}
async function start(){service=launch(resolve(root,'target/debug/supa-api'),[],root,await apiEnv());await ready(api+'/ready',service);web=launch(process.execPath,['server.mjs'],resolve(root,'reference-web'),{...base,PORT:String(sitePort),SUPA_SITE_ENV_FILE:resolve(fixtureRoot,'site.env')});await ready(site+'/healthz',web);}
async function health(state){const h=await (await fetch(site+'/api/status')).json();assert.equal(h.epoch.id,config.epoch_id);assert.equal(h.epoch.state,state);assert.equal(h.historical_database,'unrecovered');assert.deepEqual(await (await fetch(site+'/api/agents')).json(),[]);return h;}
let page;const views=['home','agents','intelligence','memory','economy','social','trading','token-launch','factory','about'];
async function inspectViews(state){for(const size of [{width:1440,height:1000},{width:390,height:844}]){await page.setViewportSize(size);await page.emulateMedia({reducedMotion:'reduce'});for(const id of views){await page.goto(site+'/site/#'+id,{waitUntil:'domcontentloaded'});await page.locator('.buffer[data-typing-state=complete]').waitFor();const text=await page.locator('.buffer').innerText();assert.ok(text.includes(config.epoch_id));assert.ok(text.includes(state==='active_read_only'?'Active, read-only':'Prepared, read-only'));assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));assert.equal(await page.locator('.autonomy-form').count(),0);}}await page.goto(site+'/site/#intelligence');await page.locator('.buffer[data-typing-state=complete]').waitFor();}
async function waitReview(stage){if(!process.env.SUPA_REA_CDP_PORT)return;console.log(JSON.stringify({stage,site,cdp_endpoint:`http://127.0.0.1:${process.env.SUPA_REA_CDP_PORT}`,fixture_only:true}));await new Promise(r=>process.once('SIGUSR1',r));}
try{
 const activeEnv=await apiEnv();
 for(const change of [{SUPA_EPOCH_APPROVAL_ID:''},{SUPA_EPOCH_APPROVAL_ID:'00000000-0000-0000-0000-000000000000'},{SUPA_ALLOW_OPENAI:'true'},{SUPA_API_SCHEDULER_ENABLED:'true'},{SUPA_WALLET_ENABLED:'true'}]){const result=spawnSync(resolve(root,'target/debug/supa-api'),[],{cwd:root,env:{...activeEnv,...change},stdio:'ignore',timeout:6000});assert.equal(result.error,undefined);assert.notEqual(result.status,0);}
 const worker=spawnSync(resolve(root,'target/debug/supa-runtime'),[],{cwd:root,env:{...activeEnv,SUPA_WORKER_ENABLED:'true'},stdio:'ignore',timeout:6000});assert.equal(worker.error,undefined);assert.notEqual(worker.status,0);
 await start();const activeHealth=await health('active_read_only');
 const denied=await fetch(api+'/agents',{method:'POST',headers:{Authorization:`Bearer ${token}`,'Content-Type':'application/json'},body:JSON.stringify({name:'Must not be created'})});assert.equal(denied.status,403);
 await stop(web);await stop(service);await start();assert.equal((await health('active_read_only')).epoch.approval_id,activeHealth.epoch.approval_id);
 const backup=fixture('backup-verify',fixtureRoot);
 browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,...(process.env.SUPA_REA_CDP_PORT?{args:[`--remote-debugging-port=${Number(process.env.SUPA_REA_CDP_PORT)}`,'--remote-debugging-address=127.0.0.1']}:{})});
 const context=await browser.newContext({reducedMotion:'reduce',colorScheme:'dark'});page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.name));await inspectViews('active_read_only');
 await waitReview('active_read_only');
 await stop(web);await stop(service);const rolled=fixture('rollback',fixtureRoot);
 const stale=spawnSync(resolve(root,'target/debug/supa-api'),[],{cwd:root,env:activeEnv,stdio:'ignore',timeout:6000});assert.equal(stale.error,undefined);assert.notEqual(stale.status,0);
 await start();const rollbackHealth=await health('prepared');assert.equal(rollbackHealth.epoch.revision,2);await page.waitForFunction(()=>document.querySelector('.buffer')?.textContent.includes('Prepared, read-only'),{},{timeout:35000});await inspectViews('prepared');assert.deepEqual(errors,[]);
 const report={fixture_database:config.database,source_epoch_id:config.epoch_id,real_candidate_activated:false,primary_frontend_repointed:false,operators_granted:0,agents_created:0,live_budgets_created:0,execution_authorizations_created:0,synthetic_transition_approvals_only:true,activation:activated,rollback:rolled,backup,service_restart_preserved_active_identity:true,stale_active_configuration_rejected:true,live_poll_converged_after_rollback:true,unsafe_startup_cases_rejected:5,worker_rejected:true,mutations_denied:true,views_checked:40,all_ten_views_active_and_rolled_back:true,viewports:[{width:1440,height:1000},{width:390,height:844}],overflow:false,browser_errors:errors,paid_requests:0,provider_requests:0,uploads:0};
 await writeFile(resolve(evidence,'drill.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(report));await waitReview('rolled_back_prepared');
}finally{await browser?.close();await stop(web);await stop(service);for(const c of children)if(c.exitCode===null)c.kill('SIGTERM');}
