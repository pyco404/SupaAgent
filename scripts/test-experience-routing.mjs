#!/usr/bin/env node
/** Production Next.js routing smoke test. Synthetic browser credentials; no database/provider. */
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {randomBytes} from 'node:crypto';
import {mkdtemp,mkdir,open,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {createServer} from 'node:net';
import {chromium} from '../web/node_modules/playwright-core/index.mjs';
import {navigation} from '../reference-web/platform.js';
const root=resolve(import.meta.dirname,'..');
const probe=createServer();await new Promise(r=>probe.listen(0,'127.0.0.1',r));const port=probe.address().port;await new Promise(r=>probe.close(r));
await mkdir(resolve(root,'artifacts/clean-routing'),{recursive:true});
const evidence=await mkdtemp(resolve(root,'artifacts/clean-routing/next-'));
const log=await open(resolve(evidence,'server.log'),'wx',0o600);
const password=randomBytes(32).toString('hex'),operatorPassword=randomBytes(32).toString('hex');
const site=`http://127.0.0.1:${port}`,basic=(username,value)=>'Basic '+Buffer.from(`${username}:${value}`).toString('base64');
const auth={Authorization:basic('routing-visitor',password)};
const child=spawn(process.execPath,['node_modules/next/dist/bin/next','start','--hostname','127.0.0.1','--port',String(port)],{cwd:resolve(root,'web'),stdio:['ignore',log.fd,log.fd],env:{PATH:process.env.PATH,HOME:process.env.HOME,NODE_ENV:'production',__NEXT_PROCESSED_ENV:'true',NEXT_TELEMETRY_DISABLED:'1',SUPA_SITE_MODE:'staging',SUPA_SITE_ORIGIN:site,SUPA_SITE_ENV_FILE:'',SUPA_BACKEND_URL:'',SUPA_API_TOKEN:'',SUPA_STAGING_USERNAME:'routing-visitor',SUPA_STAGING_PASSWORD:password,SUPA_WEB_OPERATORS:JSON.stringify([{username:'routing-operator',password:operatorPassword,api_token:randomBytes(32).toString('hex')}]),SUPA_ALLOW_OPENAI:'false',SUPA_CONTROLLED_INFERENCE:'false',SUPA_API_SCHEDULER_ENABLED:'false',SUPA_WORKER_ENABLED:'false',SUPA_WALLET_ENABLED:'false'}});
let browser;
try{
 let ready=false;
 for(let i=0;i<100;i++){try{ready=(await fetch(site+'/healthz')).ok;}catch{}if(ready)break;if(child.exitCode!==null)throw Error('Next.js exited before readiness');await new Promise(r=>setTimeout(r,200));}
 assert.ok(ready,'Next.js readiness');
 assert.equal((await fetch(site+'/memory')).status,401);
 assert.equal((await fetch(site+'/operator',{headers:auth,redirect:'manual'})).status,401);
 const signed=await fetch(site+'/operator',{headers:{Authorization:basic('routing-operator',operatorPassword)},redirect:'manual'});assert.equal(signed.status,302);assert.equal(signed.headers.get('location'),'/agents');
 browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
 const context=await browser.newContext({httpCredentials:{username:'routing-visitor',password},reducedMotion:'reduce'}),page=await context.newPage(),errors=[];
 page.on('pageerror',e=>errors.push(e.message));
 for(const width of [1440,390]){
  await page.setViewportSize({width,height:844});
  for(const [index,item] of navigation.entries()){
   assert.equal((await page.goto(site+item.path+'?preview=public')).status(),200);
   await page.locator('html[data-ready=true]').waitFor();if(item.id==='home')await page.locator('.enter').click();
   assert.equal(await page.locator('.wheel-item[aria-selected=true]').getAttribute('data-index'),String(index));
   await page.reload();await page.locator('html[data-ready=true]').waitFor();assert.equal(await page.locator('body').getAttribute('data-view'),'projects');
   assert.equal(new URL(page.url()).pathname,item.path);assert.equal(new URL(page.url()).search,'?preview=public');
  }
  await page.goto(site+'/site/?preview=public#memory');await page.locator('html[data-ready=true]').waitFor();assert.equal(new URL(page.url()).pathname,'/memory');assert.equal(new URL(page.url()).hash,'');
  await page.screenshot({path:resolve(evidence,`next-${width}.png`)});
 }
 assert.equal((await page.goto(site+'/missing')).status(),404);assert.match(await page.locator('h1').innerText(),/404/);
 for(const path of ['/app.js','/style.css','/public/assets/supa-agent-icon.svg'])assert.equal((await fetch(site+path,{headers:auth})).status,200);
 assert.deepEqual(errors,[]);
 const report={passed:true,host:'production_nextjs_private_staging',widths:[1440,390],routes:navigation.map(n=>n.path),refresh:true,legacy_migration:true,query_preserved:true,operator_redirect:true,public_auth_enforced:true,unknown_status:404,browser_errors:errors,database_connections:0,paid_requests:0};
 await writeFile(resolve(evidence,'result.json'),JSON.stringify(report,null,2));console.log(JSON.stringify({evidence,...report},null,2));
}finally{
 await browser?.close();
 if(child.exitCode===null&&child.signalCode===null){child.kill('SIGTERM');await new Promise(r=>child.once('exit',r));}
 await log.close();
}
