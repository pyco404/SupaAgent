/** Actual Chrome -> Next -> Rust -> disposable PG18. No production hosts allowed. */
import {spawn} from 'node:child_process';
import {randomBytes} from 'node:crypto';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
import {chromium} from '../web/node_modules/playwright/index.mjs';
const root=resolve(import.meta.dirname,'..'),out=resolve(root,'artifacts/functional-factory'),origin='http://127.0.0.1:3339';
const password=randomBytes(32).toString('hex');
assert(process.env.SUPA_FACTORY_BROWSER_TOKEN?.startsWith('factory-synthetic-'));
const accounts=['owner','viewer'].map(username=>({username,password,api_token:process.env[username==='owner'?'SUPA_FACTORY_BROWSER_TOKEN':'SUPA_FACTORY_VIEWER_TOKEN']}));
const env={PATH:process.env.PATH,HOME:process.env.HOME,NODE_ENV:'development',SUPA_SITE_MODE:'staging',SUPA_SYNTHETIC_FACTORY_ENABLED:'true',SUPA_PUBLIC_LANDING_ENABLED:'true',SUPA_SITE_ORIGIN:origin,SUPA_BACKEND_URL:'http://127.0.0.1:3351',SUPA_API_TOKEN:'',SUPA_WEB_OPERATORS:JSON.stringify(accounts)};
await mkdir(out,{recursive:true,mode:0o700});
const server=spawn(process.execPath,['node_modules/next/dist/bin/next','dev','--webpack','--hostname','127.0.0.1','--port','3339'],{cwd:resolve(root,'web'),env,stdio:['ignore','pipe','pipe'],detached:true});
let nextLog='';const scrub=chunk=>{let value=chunk.toString();for(const account of accounts)for(const secret of [account.password,account.api_token,Buffer.from(`${account.username}:${account.password}`).toString('base64')])value=value.split(secret).join('[redacted]');nextLog=(nextLog+value).slice(-100000);};server.stdout.on('data',scrub);server.stderr.on('data',scrub);
let browser;const report={passed:false,contexts:[],external_requests:0,unauthorized_operations:0,network:[],synthetic_only:true};
try{
 const deadline=Date.now()+120000;let ready=false;
 while(Date.now()<deadline){try{const r=await fetch(origin+'/healthz',{signal:AbortSignal.timeout(1500)});await r.arrayBuffer();if(r.ok){ready=true;break;}}catch{}await new Promise(r=>setTimeout(r,500));}
 assert(ready,'local_next_startup_required');
 browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true});
 for(const [name,viewport] of [['desktop',{width:1440,height:1000}],['mobile',{width:390,height:844}]]){
  const context=await browser.newContext({viewport,isMobile:name==='mobile',hasTouch:name==='mobile',serviceWorkers:'block',reducedMotion:'reduce'});
  const errors=[];await context.route('**/*',route=>{const url=new URL(route.request().url());if(url.origin!==origin){report.external_requests++;return route.abort();}if(url.pathname.startsWith('/api/')&&!url.pathname.startsWith('/api/factory/')){report.unauthorized_operations++;return route.abort();}return route.continue();});
  const page=await context.newPage();page.setDefaultNavigationTimeout(45000);page.on('response',r=>{report.network.push({path:new URL(r.url()).pathname,status:r.status()});});page.on('requestfailed',r=>{report.network.push({path:new URL(r.url()).pathname,failed:true});});page.on('pageerror',()=>errors.push('page_error'));
  assert.equal((await context.request.get(origin+'/api/factory/agents')).status(),401);
  assert.equal((await context.request.get(origin+'/my-agents')).status(),401);
  assert.equal((await context.request.post(origin+'/api/factory/session',{headers:{Origin:'https://untrusted.invalid','Content-Type':'application/json'},data:{username:'owner',password}})).status(),403);
  assert.equal((await context.request.post(origin+'/api/factory/session',{headers:{Origin:origin,'Content-Type':'application/json'},data:{username:'viewer',password}})).status(),403);
  await page.goto(origin+'/sign-in');await page.getByLabel('Synthetic account').fill('owner');await page.getByLabel('Password',{exact:true}).fill(password);const signedIn=page.waitForResponse(r=>r.url()===origin+'/api/factory/session'&&r.request().method()==='POST');await page.getByRole('button',{name:'Sign In',exact:true}).click();assert.equal((await signedIn).status(),200,'synthetic_sign_in_must_succeed');await page.waitForURL(origin+'/factory/create');
  const cookie=(await context.cookies()).find(c=>c.name==='supa_factory_session');assert(cookie?.httpOnly&&cookie.secure&&cookie.sameSite==='Strict');assert(!cookie.value.includes(password));
  await page.getByLabel('Name',{exact:true}).fill(`Synthetic ${name} assistant`);await page.getByLabel('Description',{exact:true}).fill('Disposable browser demonstration');await page.getByLabel('Purpose',{exact:true}).fill('Summarize synthetic reports for review');await page.getByLabel('Personality / voice').fill('Curious, concise and witty');await page.getByLabel('Goals / one per line, up to eight').fill('Summarize one synthetic report');await page.getByLabel('Read own memory').check();await page.getByLabel('I consent to privately storing').check();
  await page.getByRole('button',{name:'Review configuration',exact:true}).click();await page.screenshot({path:resolve(out,`${name}-factory-review.png`),fullPage:true});
  await page.getByRole('button',{name:'Create private paused agent',exact:true}).click();await page.waitForURL(origin+'/my-agents');await page.getByRole('heading',{name:`Synthetic ${name} assistant`,exact:true}).waitFor();
  for(const b of await page.locator('article button').all())assert(await b.isDisabled());
  assert(await page.getByText('Purpose qualification: pending review',{exact:false}).count()>0);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  await page.screenshot({path:resolve(out,`${name}-my-agents.png`),fullPage:true});
  assert.equal((await context.request.get(origin+'/operator')).status(),401,'factory cookie must not grant operator access');
  assert.equal((await context.request.get(origin+'/api/agents')).status(),401,'factory cookie must not expose old API');
  await page.getByRole('button',{name:'Sign out',exact:true}).click();await page.waitForURL(origin+'/sign-in');assert.equal((await context.request.get(origin+'/api/factory/agents')).status(),401);assert.deepEqual(errors,[]);report.contexts.push({name,sign_in_create_review_persistent_list:true,private_routes_protected:true,lifecycle_disabled:true,passed:true});await context.close();
 }
 assert.equal(report.external_requests,0);assert.equal(report.unauthorized_operations,0);report.passed=true;
}catch(error){report.failure_type=error.name;report.failure_message=String(error.message).replaceAll(password,'[redacted]').slice(0,500);throw error;}
finally{await browser?.close();try{process.kill(-server.pid,'SIGTERM');}catch{}await writeFile(resolve(out,'next-fixture.log'),nextLog,{mode:0o600});await writeFile(resolve(out,'browser-result.json'),JSON.stringify(report,null,2)+'\n',{mode:0o600});console.log(JSON.stringify(report));}
