#!/usr/bin/env node
/** Actual isolated Rust API + Node frontend; all credentials are ephemeral test values. */
import assert from 'node:assert/strict';
import {randomBytes} from 'node:crypto';
import {spawn} from 'node:child_process';
import {createServer} from 'node:net';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {chromium} from '../web/node_modules/playwright-core/index.mjs';
import {stopFixture} from './onboarding-smoke.mjs';
const root=resolve(import.meta.dirname,'..');
const backend=new URL(process.env.SUPA_8B_TEST_API);
assert.equal(backend.hostname,'127.0.0.1');assert.ok(backend.port&&backend.port!=='8080');
assert.equal(process.env.SUPA_8B_DISPOSABLE,'restored-schema-12-fixture');
const token=process.env.SUPA_8B_TEST_OPERATOR_TOKEN,serviceToken=process.env.SUPA_8B_TEST_SERVICE_TOKEN;
assert.ok(token?.length>=32&&serviceToken?.length>=32&&token!==serviceToken);
const evidence=resolve(process.env.SUPA_8B_EVIDENCE_DIR??'', 'browser');assert.ok(evidence.startsWith(resolve(root,'artifacts/milestone-8b')+'/'));await mkdir(evidence,{recursive:true,mode:0o700});
const username='disposable-owner',password=randomBytes(32).toString('hex');
const s=createServer();await new Promise(r=>s.listen(0,'127.0.0.1',r));const port=s.address().port;await new Promise(r=>s.close(r));
assert.notEqual(port,3200);const origin=`http://127.0.0.1:${port}`;
let web,browser;const errors=[];
try {
 web=spawn(process.execPath,['server.mjs'],{cwd:resolve(root,'reference-web'),env:{PATH:process.env.PATH,HOME:process.env.HOME,PORT:String(port),SUPA_SITE_ENV_FILE:'',SUPA_BACKEND_URL:backend.origin,SUPA_API_TOKEN:serviceToken,SUPA_WEB_OPERATORS:JSON.stringify([{username,password,api_token:token}])},stdio:'ignore'});
 for(let n=0;n<100;n++){assert.equal(web.exitCode,null);try{if((await fetch(origin+'/healthz')).ok)break;}catch{}await new Promise(r=>setTimeout(r,100));}
 assert.equal((await fetch(origin+'/api/operator-status')).status,401);
 browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:process.env.SUPA_8B_REA_PORT?[`--remote-debugging-port=${Number(process.env.SUPA_8B_REA_PORT)}`,'--remote-debugging-address=127.0.0.1']:[]});
 const context=await browser.newContext({httpCredentials:{username,password},colorScheme:'dark',reducedMotion:'reduce'}),page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));
 // Establish Basic authentication in the browser's existing context; REA never receives it.
 await page.goto(origin+'/operator');
 const routes=['/','/agents','/intelligence','/memory','/economy','/social','/trading','/token-launch','/factory','/about'];
 for(const [label,width,height] of [['desktop',1440,1000],['tablet',768,1024],['mobile',390,844]]){
  await page.setViewportSize({width,height});
  for(const route of routes){await page.goto(origin+route);if(route==='/'){await page.locator('html[data-ready=true]').waitFor();await page.keyboard.press('Enter');}await page.locator('.buffer[data-typing-state=complete]').waitFor();assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`${label} ${route} overflow`);}
  await page.goto(origin+'/intelligence');await page.locator('.buffer[data-typing-state=complete]').waitFor();
  const text=await page.locator('.buffer').innerText();
  assert.ok(text.includes('Credential  valid'));assert.ok(text.includes('active / agent_creation'));assert.ok(text.includes('No verified running execution'));
  assert.equal(await page.locator('.autonomy-panel').count(),0);
  await page.screenshot({path:resolve(evidence,label+'.png'),fullPage:true});
 }
 const response=await page.evaluate(async()=>{const r=await fetch('/api/operator-status',{cache:'no-store',credentials:'same-origin'});return {status:r.status,body:await r.json()};});assert.equal(response.status,200);
 const body=response.body;assert.equal(body.authorization.status,'active');assert.equal(body.eligibility.create_agent,true);assert.equal(body.eligibility.paid_inference,false);assert.equal(body.epoch.state,'prepared');
 assert.deepEqual(errors,[]);
 await writeFile(resolve(evidence,'results.json'),JSON.stringify({passed:true,actual_rust_api:true,disposable_database:true,viewports:3,routes:routes.length,unauthorized_status:401,permit_status:body.authorization.status,worker_running:body.worker.execution_running,epoch:body.epoch.state,errors,primary_repointed:false},null,2)+'\n',{mode:0o600});
 if(process.env.SUPA_8B_REA_PORT){
  await writeFile(resolve(evidence,'rea-ready.json'),JSON.stringify({origin,cdp_endpoint:`http://127.0.0.1:${Number(process.env.SUPA_8B_REA_PORT)}`,pid:process.pid})+'\n',{mode:0o600});
  await new Promise(resolve=>{const timer=setTimeout(resolve,240000);process.once('SIGTERM',()=>{clearTimeout(timer);resolve();});});
 }
} finally {await browser?.close();await stopFixture(web);}
