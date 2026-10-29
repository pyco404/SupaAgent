#!/usr/bin/env node
/** Read-only candidate preview on random ports. No cutover, fixture seeding, or paid transport. */
import assert from 'node:assert/strict';
import {readFile,lstat,mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {userInfo} from 'node:os';
import {randomBytes,randomUUID} from 'node:crypto';
import {createServer} from 'node:net';
import {spawn,spawnSync,execFileSync} from 'node:child_process';
import {chromium} from '../web/node_modules/playwright-core/index.mjs';
const root=resolve(import.meta.dirname,'..'),path=resolve(root,'postgres-data/development-epoch.json');
const info=await lstat(path);assert.ok(info.isFile()&&!info.isSymbolicLink()&&!(info.mode&0o077));
const config=JSON.parse(await readFile(path,'utf8'));
assert.equal(config.state,'prepared');assert.equal(config.activation_approved,false);
assert.equal(config.database,'supa_epoch_dev_'+config.id.replaceAll('-',''));
const url=new URL('postgresql:///'+config.database);
url.search=new URLSearchParams({host:resolve(root,'postgres-data/infrastructure-v1/socket'),port:'55440',user:userInfo().username}).toString();
const fingerprint=()=>execFileSync('python3',['-c',"import importlib.util; s=importlib.util.spec_from_file_location('e','scripts/development-epoch.py'); m=importlib.util.module_from_spec(s); s.loader.exec_module(m); c=m.load(); print(m.pg.fingerprint(c['database']))"],{cwd:root,encoding:'utf8'}).trim();
const before=fingerprint();
async function port(){const s=createServer();await new Promise(r=>s.listen(0,'127.0.0.1',r));const p=s.address().port;await new Promise(r=>s.close(r));return p;}
const apiPort=await port(),sitePort=await port(),api=`http://127.0.0.1:${apiPort}`,site=`http://127.0.0.1:${sitePort}`;
const token=randomBytes(32).toString('hex');
const env={PATH:process.env.PATH,HOME:process.env.HOME,DATABASE_URL:url.href,SUPA_API_TOKEN:token,SUPA_BIND:`127.0.0.1:${apiPort}`,SUPA_PROVIDER:'offline',SUPA_ALLOW_OPENAI:'false',SUPA_CONTROLLED_INFERENCE:'false',SUPA_WALLET_ENABLED:'false',SUPA_API_SCHEDULER_ENABLED:'false',SUPA_EPOCH_READONLY_PREVIEW:'true',SUPA_EXPECTED_EPOCH_ID:config.id,SUPA_SITE_ENV_FILE:'',SUPA_BACKEND_URL:api,PORT:String(sitePort)};
const binary=resolve(root,'target/debug/supa-api');
for(const change of [{SUPA_EPOCH_READONLY_PREVIEW:'false'},{SUPA_EXPECTED_EPOCH_ID:randomUUID()},{SUPA_EXPECTED_EPOCH_ID:''},{SUPA_ALLOW_OPENAI:'true'},{SUPA_API_SCHEDULER_ENABLED:'true'}]){
 const r=spawnSync(binary,[],{cwd:root,env:{...env,...change},timeout:6000,stdio:'ignore'});
 assert.equal(r.error,undefined);assert.notEqual(r.status,0,'unsafe startup accepted');
}
const worker=spawnSync(resolve(root,'target/debug/supa-runtime'),[],{cwd:root,env:{...env,SUPA_WORKER_ENABLED:'false'},timeout:6000,stdio:'ignore'});assert.equal(worker.error,undefined);assert.notEqual(worker.status,0);
const children=[];let browser;
function launch(cmd,args,cwd){const child=spawn(cmd,args,{cwd,env,stdio:'ignore'});children.push(child);return child;}
async function ready(url,child){for(let i=0;i<100;i++){assert.equal(child.exitCode,null);try{if((await fetch(url,{signal:AbortSignal.timeout(500)})).ok)return;}catch{}await new Promise(r=>setTimeout(r,100));}throw Error('preview timeout');}
try{
 const service=launch(binary,[],root);await ready(api+'/ready',service);
 const web=launch(process.execPath,['server.mjs'],resolve(root,'reference-web'));await ready(site+'/healthz',web);
 const health=await (await fetch(site+'/api/status')).json();assert.equal(health.epoch.id,config.id);assert.equal(health.epoch.state,'prepared');assert.equal(health.historical_database,'unrecovered');
 assert.deepEqual(await (await fetch(site+'/api/agents')).json(),[]);
 const blocked=await fetch(api+'/agents',{method:'POST',headers:{Authorization:`Bearer ${token}`,'Content-Type':'application/json'},body:JSON.stringify({name:'Must never be created'})});assert.equal(blocked.status,403);
 browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,...(process.env.SUPA_REA_CDP_PORT?{args:[`--remote-debugging-port=${Number(process.env.SUPA_REA_CDP_PORT)}`,'--remote-debugging-address=127.0.0.1']}:{})});
 const context=await browser.newContext({viewport:{width:1440,height:1000},reducedMotion:'reduce',colorScheme:'dark'});const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.name));
 const views=['home','agents','intelligence','memory','economy','social','trading','token-launch','factory','about'];
 for(const viewport of [{width:1440,height:1000},{width:390,height:844}]){
  await page.setViewportSize(viewport);
  for(const section of views){
   await page.goto(site+'/site/#'+section,{waitUntil:'domcontentloaded'});
   await page.locator('.buffer[data-typing-state=complete]').waitFor();
   await page.locator('.buffer').getByText(/Development epoch \//).waitFor();
   assert.ok((await page.locator('.buffer').innerText()).includes(config.id));
   assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
   assert.equal(await page.locator('.autonomy-form,.live-form,.schedule-form').count(),0);
  }
 }
 assert.deepEqual(errors,[]);assert.equal(fingerprint(),before);
 const evidence=resolve(root,process.env.SUPA_EPOCH_EVIDENCE_DIR??'artifacts/development-epoch');await mkdir(evidence,{recursive:true});
 const report={epoch_id:config.id,prepared:true,activated:false,real_rust_api:true,real_postgres:true,actual_empty_candidate:true,synthetic_agents_seeded:0,unchanged_candidate_fingerprint:true,startup_denial_cases:5,worker_startup_denied:true,authenticated_mutation_denied:true,views_checked:20,viewports:[{width:1440,height:1000},{width:390,height:844}],all_views_label_epoch:true,overflow:false,browser_errors:[],paid_requests:0,provider_requests:0,live_authorizations_created:0,primary_frontend_repointed:false};
 await writeFile(resolve(evidence,'browser-check.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(report));
 if(process.env.SUPA_REA_CDP_PORT){await page.goto(site+'/site/#intelligence');await page.locator('.buffer[data-typing-state=complete]').waitFor();console.log(JSON.stringify({site,cdp_endpoint:`http://127.0.0.1:${process.env.SUPA_REA_CDP_PORT}`}));await new Promise(r=>{process.once('SIGTERM',r);process.once('SIGINT',r);});}
}finally{await browser?.close();for(const child of children)if(child.exitCode===null)child.kill('SIGTERM');}
