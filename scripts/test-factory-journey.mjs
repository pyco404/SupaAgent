/** Actual Chrome -> Next -> Rust -> disposable PG18. No production hosts allowed. */
import {spawn} from 'node:child_process';
import {randomBytes} from 'node:crypto';
import {mkdir,writeFile,readFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
import http from 'node:http';
import {chromium} from '../web/node_modules/playwright/index.mjs';
const root=resolve(import.meta.dirname,'..'),out=resolve(root,'artifacts/functional-factory'),origin='http://127.0.0.1:3339';
const password=randomBytes(32).toString('hex');
assert(process.env.SUPA_FACTORY_BROWSER_TOKEN?.startsWith('factory-synthetic-'));
const accounts=['owner','viewer'].map(username=>({username,password,api_token:process.env[username==='owner'?'SUPA_FACTORY_BROWSER_TOKEN':'SUPA_FACTORY_VIEWER_TOKEN']}));
const env={PATH:process.env.PATH,HOME:process.env.HOME,TMPDIR:process.env.TMPDIR,NODE_ENV:'development',SUPA_SITE_MODE:'staging',SUPA_SYNTHETIC_FACTORY_ENABLED:'true',SUPA_PUBLIC_LANDING_ENABLED:'true',SUPA_SITE_ORIGIN:origin,SUPA_BACKEND_URL:'http://127.0.0.1:3351',SUPA_AGENT_AVATARS_ENABLED:process.env.SUPA_FACTORY_AVATARS==='1'?'true':'false',SUPA_API_TOKEN:process.env.SUPA_FACTORY_SERVICE_TOKEN||'',SUPA_WEB_OPERATORS:JSON.stringify(accounts)};
await mkdir(out,{recursive:true,mode:0o700});
const server=spawn(process.execPath,['node_modules/next/dist/bin/next','dev','--webpack','--hostname','127.0.0.1','--port','3339'],{cwd:resolve(root,'web'),env,stdio:['ignore','pipe','pipe'],detached:true});
let nextLog='';const scrub=chunk=>{let value=chunk.toString();for(const account of accounts)for(const secret of [account.password,account.api_token,Buffer.from(`${account.username}:${account.password}`).toString('base64')])value=value.split(secret).join('[redacted]');nextLog=(nextLog+value).slice(-100000);};server.stdout.on('data',scrub);server.stderr.on('data',scrub);
let browser,publicServer;const report={passed:false,contexts:[],external_requests:0,unauthorized_operations:0,network:[],synthetic_only:true};
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
  if(process.env.SUPA_FACTORY_AVATARS==='1'){
   assert(await page.getByRole('button',{name:'Generate with AI / unavailable',exact:true}).isDisabled());
   if(name==='desktop'){
    await page.getByLabel('Upload Image / PNG, JPG or WebP, up to 5 MB',{exact:true}).setInputFiles({name:'portrait.png',mimeType:'image/png',buffer:await readFile(resolve(root,'artifacts/avatar-tests/portrait.png'))});
    await page.locator('#crop-controls').waitFor({state:'visible'});await page.getByLabel('Crop zoom',{exact:true}).focus();await page.getByLabel('Crop zoom',{exact:true}).press('ArrowRight');
    await page.getByRole('button',{name:'Accept square crop',exact:true}).click();await page.getByText('Square image accepted. Metadata stripped; still private.',{exact:true}).waitFor();
   }
  }
  await page.getByRole('button',{name:'Review configuration',exact:true}).click();await page.screenshot({path:resolve(out,`${name}-factory-review.png`),fullPage:true});
  await page.getByRole('button',{name:'Create private paused agent',exact:true}).click();await page.waitForURL(origin+'/my-agents');await page.getByRole('heading',{name:`Synthetic ${name} assistant`,exact:true}).waitFor();
  for(const b of await page.locator('article button').all())assert(await b.isDisabled());
  assert(await page.getByText('Purpose qualification: pending review',{exact:false}).count()>0);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  await page.screenshot({path:resolve(out,`${name}-my-agents.png`),fullPage:true});
  const ownedAgents=await (await context.request.get(origin+'/api/factory/agents')).json();
  const created=ownedAgents.agents.find(a=>a.configuration.name===`Synthetic ${name} assistant`);assert(created);
  const detailPath=origin+'/api/factory/agents/'+created.agent_id;
  const detail=await context.request.get(detailPath);assert.equal(detail.status(),200);assert.deepEqual(await detail.json(),created);
  assert.equal((await context.request.post(detailPath,{headers:{Origin:origin,'Content-Type':'application/json'},data:{name:'Unauthorized edit'}})).status(),404);
  const unknown=await context.request.get(origin+'/api/factory/agents/99999999-9999-4999-8999-999999999999');assert.equal(unknown.status(),404);
  if(process.env.SUPA_FACTORY_AVATARS!=='1'){
   await page.getByRole('link',{name:'View details / configuration',exact:true}).last().click();await page.waitForURL(/\/my-agents\/[0-9a-f-]{36}$/);
   await page.getByRole('heading',{name:`Synthetic ${name} assistant`,exact:true}).waitFor();
   assert.equal(await page.locator('article').count(),1);assert.equal(await page.locator('article button:not(:disabled)').count(),0);
  }
  if(process.env.SUPA_FACTORY_AVATARS==='1'){
   const article=page.locator('article').filter({has:page.getByRole('heading',{name:`Synthetic ${name} assistant`,exact:true})});
   await article.getByRole('link',{name:'View profile / edit avatar',exact:true}).click();await page.waitForURL(/\/my-agents\/[0-9a-f-]{36}$/);
   assert.equal(await page.locator('article').count(),1,'single owner-authorized detail record');
   if(name==='mobile'){const meta=await (await context.request.get(origin+'/api/factory/avatars/'+page.url().split('/').pop())).json();assert.equal(meta.revision,0);assert.equal(meta.asset_id,null);}
   await page.locator('#avatar-preview').evaluate(img=>img.decode());assert.equal(await page.locator('#avatar-preview').evaluate(img=>img.naturalWidth),512);
   if(name==='desktop'){
    await page.getByRole('button',{name:'Save profile avatar',exact:true}).click();await page.getByText('No avatar changes to save.',{exact:true}).waitFor();
    await page.getByLabel('Upload Image / PNG, JPG or WebP, up to 5 MB',{exact:true}).setInputFiles({name:'replacement.png',mimeType:'image/png',buffer:await readFile(resolve(root,'artifacts/avatar-tests/first.png'))});
    await page.locator('#crop-controls').waitFor({state:'visible'});await page.getByRole('button',{name:'Accept square crop',exact:true}).click();await page.getByText('Square image accepted. Metadata stripped; still private.',{exact:true}).waitFor();
    await page.getByRole('button',{name:'Save profile avatar',exact:true}).click();await page.getByText('Profile avatar saved. Any confirmed token image is unchanged.',{exact:true}).waitFor();
    await page.getByLabel('I explicitly confirm the currently saved image').check();await page.getByRole('button',{name:'Confirm token image / no launch',exact:true}).click();await page.getByText('Token image version and checksum preserved. No token launched.',{exact:true}).waitFor();
    const frozen=await page.locator('#token-image-status').textContent();assert(frozen.includes('revision 2'));
    await page.getByRole('button',{name:'Use Default Avatar',exact:true}).click();await page.getByText('Default avatar selected. No payment or AI service required.',{exact:true}).waitFor();
    await page.getByRole('button',{name:'Save profile avatar',exact:true}).click();await page.getByText('Profile avatar saved. Any confirmed token image is unchanged.',{exact:true}).waitFor();assert.equal(await page.locator('#token-image-status').textContent(),frozen);
   }
   assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
   await page.screenshot({path:resolve(out,`${name}-avatar-profile.png`),fullPage:true});
  }
  assert.equal((await context.request.get(origin+'/operator')).status(),401,'factory cookie must not grant operator access');
  assert.equal((await context.request.get(origin+'/api/agents')).status(),401,'factory cookie must not expose old API');
  await page.getByRole('button',{name:'Sign out',exact:true}).click();await page.waitForURL(origin+'/sign-in');assert.equal((await context.request.get(origin+'/api/factory/agents')).status(),401);assert.deepEqual(errors,[]);report.contexts.push({name,sign_in_create_review_persistent_list:true,private_routes_protected:true,lifecycle_disabled:true,passed:true});await context.close();
 }
 if(process.env.SUPA_FACTORY_AVATARS==='1'){
  const publicOrigin='http://127.0.0.1:3340';
  publicServer=http.createServer((req,res)=>{if(!['GET','HEAD'].includes(req.method)||!req.url.startsWith('/')||req.url.startsWith('//')){res.writeHead(405).end();return;}const headers={...req.headers,host:'supaagency.xyz'};delete headers.cookie;delete headers.authorization;if(headers.origin===publicOrigin)headers.origin='https://supaagency.xyz';const upstream=http.request({host:'127.0.0.1',port:3339,path:req.url,method:req.method,headers},response=>{res.writeHead(response.statusCode,response.headers);response.pipe(res);});upstream.on('error',()=>res.writeHead(502).end());upstream.end();});
  await new Promise(r=>publicServer.listen(3340,'127.0.0.1',r));
  const context=await browser.newContext({viewport:{width:390,height:844},serviceWorkers:'block'});await context.route('**/*',route=>{const url=new URL(route.request().url());if(url.origin!==publicOrigin||!['GET','HEAD'].includes(route.request().method())||(url.pathname.startsWith('/api/')&&!url.pathname.startsWith('/api/public/'))){report.unauthorized_operations++;return route.abort();}return route.continue();});
  const page=await context.newPage();await page.goto(publicOrigin+'/agents');const link=page.locator('a[href^="/agents/"]').first();await link.waitFor();await link.click();await page.waitForURL(/\/agents\/[0-9a-f-]{36}$/);await page.getByRole('heading',{name:'Synthetic research assistant',exact:true}).waitFor();await page.locator('#avatar-slot img').evaluate(img=>img.decode());assert.equal(await page.locator('#avatar-slot img').evaluate(img=>img.naturalWidth),512);assert(!(await page.locator('body').textContent()).includes('PRIVATE_PURPOSE_DO_NOT_DISCLOSE'));assert.equal((await context.request.get(publicOrigin+'/my-agents')).status(),401);await page.screenshot({path:resolve(out,'public-avatar-profile.png'),fullPage:true});await context.close();report.public_avatar_profile=true;
 }
 assert.equal(report.external_requests,0);assert.equal(report.unauthorized_operations,0);report.passed=true;
}catch(error){report.failure_type=error.name;let details=String(error.message);for(const account of accounts)for(const secret of [account.password,account.api_token])details=details.split(secret).join('[redacted]');report.failure_message=details.slice(0,500);throw new Error('synthetic_browser_acceptance_failed');}
finally{await browser?.close();if(publicServer)await new Promise(r=>publicServer.close(r));try{process.kill(-server.pid,'SIGTERM');}catch{}await writeFile(resolve(out,'next-fixture.log'),nextLog,{mode:0o600});await writeFile(resolve(out,'browser-result.json'),JSON.stringify(report,null,2)+'\n',{mode:0o600});console.log(JSON.stringify(report));}
