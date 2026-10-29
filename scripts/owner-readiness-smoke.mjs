#!/usr/bin/env node
/** Real API/DB/site drill; synthetic owner only, never enables the pending real identity. */
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {parseEnv} from 'node:util';
import {resolve} from 'node:path';
import {randomBytes} from 'node:crypto';
import {spawn,spawnSync,execFileSync} from 'node:child_process';
import {createServer} from 'node:net';
import {chromium} from '../web/node_modules/playwright-core/index.mjs';
const root=resolve(import.meta.dirname,'..'),out=resolve(root,'artifacts/owner-readiness');await mkdir(out,{recursive:true});
const fixture=(action,path)=>JSON.parse(execFileSync('python3',['scripts/owner-cutover-fixture.py',action,...(path?[path]:[])],{cwd:root,encoding:'utf8'}));
const prepared=fixture('prepare'),folder=prepared.root,c=JSON.parse(await readFile(resolve(folder,'owner-fixture.json'),'utf8'));
async function port(){const s=createServer();await new Promise(r=>s.listen(0,'127.0.0.1',r));const n=s.address().port;await new Promise(r=>s.close(r));return n;}
const apiPort=await port(),sitePort=await port(),api=`http://127.0.0.1:${apiPort}`,site=`http://127.0.0.1:${sitePort}`;
const database=execFileSync('python3',['-c',`import importlib.util;s=importlib.util.spec_from_file_location('e','scripts/development-epoch.py');m=importlib.util.module_from_spec(s);s.loader.exec_module(m);print(m.url('${c.database}'))`],{cwd:root,encoding:'utf8'}).trim();
const serviceToken=randomBytes(32).toString('hex'),operatorToken=randomBytes(32).toString('hex'),password=randomBytes(32).toString('hex');
const basic='Basic '+Buffer.from('fixture-owner:'+password).toString('base64');
const lines=values=>Object.entries(values).map(([k,v])=>{assert.ok(!/[\n\r\0']/.test(v));return k+"='"+v+"'";}).join('\n')+'\n';
const original={DATABASE_URL:database,SUPA_BIND:`127.0.0.1:${apiPort}`,SUPA_API_TOKEN:serviceToken,SUPA_PROVIDER:'offline',SUPA_ALLOW_OPENAI:'false',SUPA_CONTROLLED_INFERENCE:'false',SUPA_API_SCHEDULER_ENABLED:'false',SUPA_WALLET_ENABLED:'false',SUPA_PRIVATE_STAGING:'false',SUPA_EXPECTED_EPOCH_ID:c.epoch_id,SUPA_EPOCH_READONLY_PREVIEW:'true',SUPA_EPOCH_MODE:'preview',SUPA_OPERATOR_CREDENTIALS:JSON.stringify([{id:c.synthetic_actor_id,role:'owner',token:operatorToken}])};
await writeFile(resolve(folder,'api.env'),lines(original),{flag:'wx',mode:0o600});
await writeFile(resolve(folder,'site.env'),lines({SUPA_BACKEND_URL:api,SUPA_API_TOKEN:serviceToken,SUPA_WEB_OPERATORS:JSON.stringify([{username:'fixture-owner',password,api_token:operatorToken}])}),{flag:'wx',mode:0o600});
const activation=fixture('activate',folder),base={PATH:process.env.PATH,HOME:process.env.HOME};
let service,web,browser;const children=[];
function launch(cmd,args,cwd,env){const p=spawn(cmd,args,{cwd,env,stdio:'ignore'});children.push(p);return p;}
async function stop(p){if(p?.exitCode===null){p.kill('SIGTERM');await new Promise(r=>p.once('exit',r));}}
async function ready(url,p){for(let n=0;n<100;n++){assert.equal(p.exitCode,null);try{if((await fetch(url,{signal:AbortSignal.timeout(500)})).ok)return;}catch{}await new Promise(r=>setTimeout(r,100));}throw Error('readiness timeout');}
const apiEnv=async()=>({...base,...parseEnv(await readFile(resolve(folder,'api.env'),'utf8'))});
async function start(){service=launch(resolve(root,'target/debug/supa-api'),[],root,await apiEnv());await ready(api+'/ready',service);web=launch(process.execPath,['server.mjs'],resolve(root,'reference-web'),{...base,PORT:String(sitePort),SUPA_SITE_ENV_FILE:resolve(folder,'site.env')});await ready(site+'/healthz',web);}
const bearer=token=>({Authorization:'Bearer '+token});
async function health(state){const h=await (await fetch(site+'/api/status')).json();assert.equal(h.epoch.id,c.epoch_id);assert.equal(h.epoch.state,state);assert.equal(h.historical_database,'unrecovered');return h;}
async function review(stage){if(!process.env.SUPA_REA_CDP_PORT)return;console.log(JSON.stringify({stage,site,cdp_endpoint:`http://127.0.0.1:${process.env.SUPA_REA_CDP_PORT}`,synthetic_owner_only:true}));await new Promise(r=>process.once('SIGUSR1',r));}
const views=['home','agents','intelligence','memory','economy','social','trading','token-launch','factory','about'];
try{
 await start();const active=await health('active_read_only');
 assert.equal((await fetch(api+'/agents',{headers:bearer(operatorToken)})).status,200);
 assert.deepEqual(await (await fetch(api+'/agents',{headers:bearer(operatorToken)})).json(),[]);
 for(const token of [serviceToken,'unknown-fixture-token']){
  const r=await fetch(api+'/agents',{headers:{...bearer(token),'x-supa-principal':c.synthetic_actor_id,'x-supa-role':'owner'}});assert.equal(r.status,token===serviceToken?403:401);
 }
 // Real dormant credential is tested only for rejection. It is never installed or sent to a browser.
 const pending=JSON.parse(await readFile(resolve(root,'postgres-data/owner-provisioning/credentials.pending.json'),'utf8'));
 assert.equal((await fetch(api+'/agents',{headers:bearer(pending.operator_token)})).status,401);
 assert.equal((await fetch(site+'/operator',{headers:{Authorization:'Basic '+Buffer.from(pending.username+':'+pending.password).toString('base64')},redirect:'manual'})).status,401);
 for(const path of ['/agents','/autonomy','/agents/00000000-0000-4000-8000-000000000001/access']){
  assert.equal((await fetch(api+path,{method:'POST',headers:{...bearer(operatorToken),'Content-Type':'application/json'},body:'{}'})).status,403);
 }
 assert.equal((await fetch(site+'/api/autonomy',{headers:{'x-supa-principal':c.synthetic_actor_id,'x-supa-role':'owner'}})).status,401);
 assert.equal((await fetch(site+'/api/autonomy',{headers:{Authorization:basic}})).status,200);
 const spoof=await fetch(site+'/api/autonomy',{method:'POST',headers:{Authorization:basic,Origin:site,'x-supa-action':'offline-autonomy','Content-Type':'application/json'},body:JSON.stringify({request_id:'00000000-0000-4000-8000-000000000001',goal:'This request must be denied',authorize_offline:true})});assert.equal(spoof.status,403);
 await stop(web);await stop(service);await start();assert.equal((await health('active_read_only')).epoch.approval_id,active.epoch.approval_id);
 const activeEnv=await apiEnv();for(const flags of [{SUPA_ALLOW_OPENAI:'true'},{SUPA_CONTROLLED_INFERENCE:'true'},{SUPA_API_SCHEDULER_ENABLED:'true'},{SUPA_WALLET_ENABLED:'true'}]){const r=spawnSync(resolve(root,'target/debug/supa-api'),[],{cwd:root,env:{...activeEnv,...flags},stdio:'ignore',timeout:6000});assert.equal(r.error,undefined);assert.notEqual(r.status,0);}
 const worker=spawnSync(resolve(root,'target/debug/supa-runtime'),[],{cwd:root,env:{...activeEnv,SUPA_WORKER_ENABLED:'true'},stdio:'ignore',timeout:6000});assert.equal(worker.error,undefined);assert.notEqual(worker.status,0);
 const backup=fixture('backup-verify',folder);
 browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,...(process.env.SUPA_REA_CDP_PORT?{args:[`--remote-debugging-port=${Number(process.env.SUPA_REA_CDP_PORT)}`,'--remote-debugging-address=127.0.0.1']}:{})});
 const context=await browser.newContext({httpCredentials:{username:'fixture-owner',password},reducedMotion:'reduce'});const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.name));await page.goto(site+'/operator');
 for(const size of [{width:1440,height:1000},{width:768,height:1024},{width:390,height:844}]){
  await page.setViewportSize(size);for(const id of views){await page.goto(site+'/site/#'+id);await page.locator('.buffer[data-typing-state=complete]').waitFor();const text=await page.locator('.buffer').innerText();assert.ok(text.includes(c.epoch_id));assert.ok(text.includes('Active, read-only'));assert.equal(await page.locator('.autonomy-form').count(),0);assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));}
 }
 await page.setViewportSize({width:1440,height:1000});await page.goto(site+'/site/#intelligence');await page.locator('.buffer[data-typing-state=complete]').waitFor();await review('active_desktop');
 await page.setViewportSize({width:390,height:844});await page.goto(site+'/site/#about');await page.locator('.buffer[data-typing-state=complete]').waitFor();await review('active_mobile');
 await stop(web);await stop(service);const rollback=fixture('rollback',folder);await start();assert.equal((await health('prepared')).epoch.revision,2);await page.waitForFunction(()=>document.querySelector('.buffer')?.textContent.includes('Prepared, read-only'),{},{timeout:35000});
 for(const id of views){await page.goto(site+'/site/#'+id);await page.locator('.buffer[data-typing-state=complete]').waitFor();assert.ok((await page.locator('.buffer').innerText()).includes('Prepared, read-only'));}
 const stale=spawnSync(resolve(root,'target/debug/supa-api'),[],{cwd:root,env:activeEnv,stdio:'ignore',timeout:6000});assert.equal(stale.error,undefined);assert.notEqual(stale.status,0);assert.deepEqual(errors,[]);
 const report={fixture_database:c.database,epoch_id:c.epoch_id,synthetic_owner_only:true,real_owner_granted:false,real_credentials_installed:false,real_pending_credentials_rejected:true,primary_frontend_repointed:false,real_candidate_activated:false,activation,rollback,backup,restart_verified:true,spoofed_identity_rejected:true,shared_service_cannot_be_owner:true,owner_mutations_denied:true,unauthorized_browser_access_rejected:true,unsafe_flags_rejected:4,worker_rejected:true,stale_active_profile_rejected:true,active_view_checks:30,rollback_view_checks:10,viewports:[1440,768,390],browser_errors:errors,agents_created:0,agent_ownership_grants:0,execution_authorizations:0,new_budgets:0,paid_requests:0,external_mutations:0,uploads:0};
 await writeFile(resolve(out,'drill.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(report));
}finally{await browser?.close();await stop(web);await stop(service);for(const p of children)if(p.exitCode===null)p.kill('SIGTERM');}
