#!/usr/bin/env node
/** Browser validation of the approved UI against a disposable, least-privilege backend. */
import assert from 'node:assert/strict';
import {readFile,writeFile,lstat} from 'node:fs/promises';
import {resolve} from 'node:path';
import {spawn,spawnSync} from 'node:child_process';
import {createServer} from 'node:net';
import {chromium} from '../web/node_modules/playwright-core/index.mjs';
const root=resolve(import.meta.dirname,'..'),file=resolve(process.argv[2]??'');
const remediation=process.env.SUPA_VALIDATION_SUITE==='activation-remediation',fixtureDirectory=remediation?'remediation':'safety',evidenceDirectory=remediation?'activation-remediation':'final-local-safety';
assert.match(file,new RegExp('^'+root.replace(/[.*+?^${}()|[\]\\]/g,'\\$&')+'/postgres-data/'+fixtureDirectory+'/[a-f0-9]{12}/runtime\\.json$'));
const info=await lstat(file);assert.equal(info.isSymbolicLink(),false);assert.equal(info.mode&0o077,0);assert.equal(info.uid,process.getuid());
const c=JSON.parse(await readFile(file,'utf8'));assert.ok(c.database.startsWith('supa_cutover_test_'));
async function port(){const s=createServer();await new Promise(r=>s.listen(0,'127.0.0.1',r));const n=s.address().port;await new Promise(r=>s.close(r));return n;}
const apiPort=await port(),sitePort=await port(),api=`http://127.0.0.1:${apiPort}`,site=`http://127.0.0.1:${sitePort}`;
const base={PATH:process.env.PATH,HOME:process.env.HOME},children=[];let browser,service,web;
const apiEnv={...base,...c.api,SUPA_BIND:`127.0.0.1:${apiPort}`};
const siteEnv={...base,PORT:String(sitePort),SUPA_SITE_ENV_FILE:'',SUPA_BACKEND_URL:api,SUPA_API_TOKEN:c.api.SUPA_API_TOKEN,SUPA_WEB_OPERATORS:JSON.stringify([{username:c.synthetic_username,password:c.synthetic_password,api_token:c.synthetic_token}])};
for(const key of ['SUPA_OPERATOR_ID','SUPA_WEB_USERNAME','SUPA_WEB_PASSWORD','OPENAI_API_KEY']){assert.equal(apiEnv[key],undefined);assert.equal(siteEnv[key],undefined);}
assert.notEqual(c.synthetic_token,c.api.SUPA_API_TOKEN);
function launch(cmd,args,cwd,env){const p=spawn(cmd,args,{cwd,env,stdio:'ignore'});children.push(p);return p;}
async function stop(p){if(p?.exitCode===null){p.kill('SIGTERM');await new Promise(r=>p.once('exit',r));}}
async function ready(url,p){for(let i=0;i<100;i++){assert.equal(p.exitCode,null);try{if((await fetch(url,{signal:AbortSignal.timeout(500)})).ok)return;}catch{}await new Promise(r=>setTimeout(r,100));}throw Error('isolated service not ready');}
async function start(){service=launch(resolve(root,'target/debug/supa-api'),[],root,apiEnv);await ready(api+'/ready',service);web=launch(process.execPath,['server.mjs'],resolve(root,'reference-web'),siteEnv);await ready(site+'/healthz',web);}
async function pause(stage){if(process.env.SUPA_REA_CDP_PORT){console.log(JSON.stringify({stage,site,cdp_endpoint:`http://127.0.0.1:${process.env.SUPA_REA_CDP_PORT}`,fixture_only:true}));await new Promise(r=>process.once('SIGUSR1',r));}}
try{
 await start();const h=await (await fetch(site+'/api/status')).json();assert.equal(h.epoch.id,c.epoch_id);assert.equal(h.epoch.state,'active_read_only');assert.equal(h.epoch.approval_id,c.approval_id);
 assert.deepEqual(await (await fetch(site+'/api/agents')).json(),[]);
 const basic='Basic '+Buffer.from(c.synthetic_username+':'+c.synthetic_password).toString('base64');
 assert.equal((await fetch(site+'/operator',{redirect:'manual'})).status,401);
 assert.equal((await fetch(site+'/operator',{redirect:'manual',headers:{Authorization:basic}})).status,302);
 assert.equal((await fetch(site+'/api/autonomy',{headers:{'x-supa-role':'owner','x-supa-principal':c.epoch_id}})).status,401);
 assert.equal((await fetch(api+'/agents',{headers:{Authorization:'Bearer '+c.api.SUPA_API_TOKEN,'x-supa-role':'owner'}})).status,403);
 assert.equal((await fetch(api+'/agents',{headers:{Authorization:'Bearer '+c.synthetic_token}})).status,200);
 assert.equal((await fetch(api+'/agents',{method:'POST',headers:{Authorization:'Bearer '+c.synthetic_token,'Content-Type':'application/json'},body:'{}'})).status,403);
 await stop(web);await stop(service);await start();assert.equal((await (await fetch(site+'/api/status')).json()).epoch.approval_id,c.approval_id);
 for(const flags of [{SUPA_API_SCHEDULER_ENABLED:'true'},{SUPA_ALLOW_OPENAI:'true'},{SUPA_CONTROLLED_INFERENCE:'true'},{SUPA_WALLET_ENABLED:'true'}]){const r=spawnSync(resolve(root,'target/debug/supa-api'),[],{cwd:root,env:{...apiEnv,...flags},stdio:'ignore',timeout:6000});assert.equal(r.error,undefined);assert.notEqual(r.status,0);}
 const wrongCredential=new URL(c.api.DATABASE_URL);wrongCredential.password='wrong-fixture-password';
 for(const databaseUrl of ['postgresql://',wrongCredential.href]){const r=spawnSync(resolve(root,'target/debug/supa-api'),[],{cwd:root,env:{...apiEnv,DATABASE_URL:databaseUrl},stdio:'ignore',timeout:10000});assert.equal(r.error,undefined);assert.notEqual(r.status,0);}
 const worker=spawnSync(resolve(root,'target/debug/supa-runtime'),[],{cwd:root,env:{...apiEnv,SUPA_WORKER_ENABLED:'true'},stdio:'ignore',timeout:6000});assert.equal(worker.error,undefined);assert.notEqual(worker.status,0);
 browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,...(process.env.SUPA_REA_CDP_PORT?{args:[`--remote-debugging-port=${Number(process.env.SUPA_REA_CDP_PORT)}`,'--remote-debugging-address=127.0.0.1']}:{})});
 const context=await browser.newContext({httpCredentials:{username:c.synthetic_username,password:c.synthetic_password},reducedMotion:'reduce'}),page=await context.newPage(),errors=[];
 page.on('pageerror',e=>errors.push(e.name));await page.goto(site+'/operator');
 const views=['home','agents','intelligence','memory','economy','social','trading','token-launch','factory','about'];
 for(const viewport of [{width:1440,height:1000},{width:768,height:1024},{width:390,height:844}]){await page.setViewportSize(viewport);for(const id of views){await page.goto(site+'/site/#'+id);await page.locator('.buffer[data-typing-state=complete]').waitFor();const text=await page.locator('.buffer').innerText();assert.ok(text.includes(c.epoch_id));assert.ok(text.includes('Active, read-only'));assert.equal(await page.locator('.autonomy-form').count(),0);assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));}}
 await page.setViewportSize({width:1440,height:1000});await page.goto(site+'/site/#intelligence');await page.locator('.buffer[data-typing-state=complete]').waitFor();await pause('desktop');
 await page.setViewportSize({width:390,height:844});await page.goto(site+'/site/#about');await page.locator('.buffer[data-typing-state=complete]').waitFor();await pause('mobile');
 assert.deepEqual(errors,[]);
 await writeFile(resolve(root,'artifacts',evidenceDirectory,'browser.json'),JSON.stringify({disposable_cluster_only:true,synthetic_credentials_only:true,actual_scram_select_only_backend:true,view_checks:30,viewports:[1440,768,390],browser_errors:errors,overflow:false,unauthorized_and_spoofed_identity_denied:true,shared_service_cannot_be_owner:true,owner_mutations_denied:true,unsafe_startup_flags_rejected:4,invalid_connection_startups_rejected:2,worker_rejected:true,restart_preserved_approval:true,primary_frontend_repointed:false,real_credentials_installed:false,paid_requests:0},null,2)+'\n',{flag:'wx'});
}finally{await browser?.close();await stop(web);await stop(service);for(const p of children)if(p.exitCode===null)p.kill('SIGTERM');}
