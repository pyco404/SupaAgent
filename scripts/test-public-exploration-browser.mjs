#!/usr/bin/env node
/** Read-only actual Next.js browser rehearsal. All accounts/backend records are synthetic. */
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {randomBytes} from 'node:crypto';
import {createServer,request} from 'node:http';
import {mkdir,writeFile,mkdtemp,rm} from 'node:fs/promises';
import {resolve} from 'node:path';
import {chromium} from '../web/node_modules/playwright/index.mjs';

const web=process.env.SUPA_TEST_WEB_ROOT;assert.ok(web,'isolated_web_root_required');
const output=resolve(process.env.SUPA_TEST_EVIDENCE??'artifacts/public-exploration');
await mkdir(output,{recursive:true,mode:0o700});
const secret=()=>randomBytes(32).toString('hex');
const accounts=['operator','viewer'].map(role=>({role,username:`test-${role}`,password:secret(),api_token:secret()}));
const visitor={username:'test-visitor',password:secret()},serviceToken=secret();
const basic=a=>'Basic '+Buffer.from(`${a.username}:${a.password}`).toString('base64');
const sentinel='PRIVATE_FIXTURE_DO_NOT_PUBLISH';let backendReads=0,backendWrites=0;
const backend=createServer((req,res)=>{
 res.setHeader('Content-Type','application/json');
 if(req.method!=='GET'){backendWrites++;res.writeHead(403).end('{}');return;}
 backendReads++;const role=accounts.find(a=>req.headers.authorization===`Bearer ${a.api_token}`)?.role;
 const service=req.headers.authorization===`Bearer ${serviceToken}`;
 if(!role&&!service){res.writeHead(401).end('{}');return;}
 if(req.url==='/directory'){res.end(JSON.stringify([{id:'11111111-1111-4111-8111-111111111111',name:sentinel,lifecycle:'paused',created_at:'2026-01-01T00:00:00Z'}]));return;}
 if(req.url==='/health'){res.end(JSON.stringify({status:'ok',economic_mode:'simulation',publishing_enabled:false}));return;}
 if(req.url==='/operator-status'&&role){res.end(JSON.stringify({database:'synthetic_fixture',schema_version:13,database_read_only_default:true,checked_at:new Date().toISOString(),epoch:null,role,api_session_expires_at:null,credential:{status:'valid',expires_at:null},authorization:{status:'inactive',scope:null,agent_id:null,expires_at:null,max_cycles:null,budget_microusd:null},worker:{status:'unavailable',execution_running:false,heartbeat_at:null,lease_until:null},eligibility:{create_agent:false,grant_owner:false,activate_offline:false,execute_offline:false,paid_inference:false,financial_operations:false,reason:'Read-only synthetic test'}}));return;}
 res.writeHead(403).end('{}');
});
await new Promise(r=>backend.listen(0,'127.0.0.1',r));
const portProbe=createServer();await new Promise(r=>portProbe.listen(0,'127.0.0.1',r));const port=portProbe.address().port;await new Promise(r=>portProbe.close(r));
const origin=`http://127.0.0.1:${port}`;
// A local-only host adapter exercises the exact official Host boundary without
// asking Chromium to send the forbidden Host override header.
const hostAdapter=createServer((req,res)=>{
 const headers={...req.headers,host:'supaagency.xyz'};
 // Module/font requests carry Origin. Translate only this local browser origin;
 // foreign origins remain unchanged and must still be rejected by the app.
 if(headers.origin===browserOrigin)headers.origin='https://supaagency.xyz';
 const upstream=request(origin+req.url,{method:req.method,headers},reply=>{res.writeHead(reply.statusCode,reply.headers);reply.pipe(res);});
 upstream.on('error',()=>res.writeHead(502).end());req.pipe(upstream);
});
await new Promise(r=>hostAdapter.listen(0,'127.0.0.1',r));const browserOrigin=`http://127.0.0.1:${hostAdapter.address().port}`;
const next=spawn(process.execPath,['node_modules/next/dist/bin/next','dev','--webpack','--hostname','127.0.0.1','--port',String(port)],{cwd:web,stdio:'ignore',env:{PATH:'/usr/local/bin:/usr/bin:/bin',NODE_ENV:'development',NEXT_TELEMETRY_DISABLED:'1',__NEXT_PROCESSED_ENV:'true',SUPA_SITE_MODE:'staging',SUPA_SITE_ORIGIN:origin,SUPA_PUBLIC_LANDING_ENABLED:'true',SUPA_SITE_ENV_FILE:'',SUPA_BACKEND_URL:`http://127.0.0.1:${backend.address().port}`,SUPA_API_TOKEN:serviceToken,SUPA_WEB_OPERATORS:JSON.stringify(accounts),SUPA_STAGING_USERNAME:visitor.username,SUPA_STAGING_PASSWORD:visitor.password}});
const get=(path,headers={})=>new Promise((yes,no)=>{const req=request(origin+path,{headers:{host:'supaagency.xyz',...headers},timeout:60000},res=>{let body='';res.on('data',b=>{body+=b;if(body.length>2_000_000)req.destroy(Error('body_bound'));});res.on('end',()=>yes({status:res.statusCode,body,headers:res.headers}));});req.on('timeout',()=>req.destroy(Error('request_timeout')));req.on('error',no);req.end();});
let browser,child,profile,stage='next_readiness';const report={passed:false,actual_next_development_server:true,production_image_built:false,synthetic_backend:true,contexts:[],roles:[],page_errors:[]};
try{
 let ready=false;for(let i=0;i<120;i++){try{ready=(await get('/healthz')).status===200;}catch{}if(ready||next.exitCode!==null)break;await new Promise(r=>setTimeout(r,250));}assert.ok(ready,'next_ready');
 profile=await mkdtemp('/dev/shm/supa-exploration-chrome-');
 const probe=createServer();await new Promise(r=>probe.listen(0,'127.0.0.1',r));const cdpPort=probe.address().port;await new Promise(r=>probe.close(r));
 child=spawn(process.env.SUPA_TEST_CHROMIUM??'/home/pikoo/.cache/ms-playwright/chromium_headless_shell-1243/chrome-headless-shell-linux64/chrome-headless-shell',['--no-sandbox','--disable-gpu',`--remote-debugging-port=${cdpPort}`,'--remote-debugging-address=127.0.0.1',`--user-data-dir=${profile}`,'--no-first-run','--disable-background-networking','--disable-sync','--disable-component-update','about:blank'],{stdio:'ignore'});
 const cdp=`http://127.0.0.1:${cdpPort}`;for(let i=0;i<80;i++){try{if((await fetch(cdp+'/json/version')).ok)break;}catch{}await new Promise(r=>setTimeout(r,100));}
 browser=await chromium.connectOverCDP(cdp);
 for(const test of [{name:'desktop',viewport:{width:1440,height:1000}},{name:'mobile',viewport:{width:390,height:844},isMobile:true,hasTouch:true}]){
  const before=backendReads;
  stage=test.name+'_public_browser';
  const context=await browser.newContext({...test,reducedMotion:'reduce',serviceWorkers:'block'});
  let apiPolls=0,mutations=0;await context.route('**/*',route=>{const q=route.request(),url=new URL(q.url());if(!['GET','HEAD'].includes(q.method())){mutations++;return route.abort();}if(url.pathname.startsWith('/api/'))apiPolls++;if(url.origin!==browserOrigin)return route.abort();return route.continue();});
  const page=await context.newPage();page.on('pageerror',e=>report.page_errors.push(e.name));
  stage=test.name+'_landing_navigation';assert.equal((await page.goto(browserOrigin)).status(),200,'anonymous_landing');
  stage=test.name+'_scripts_ready';await page.locator('html[data-ready=true]').waitFor();
  stage=test.name+'_enter_click';assert.equal(await page.locator('.enter').getAttribute('href'),'/home');await page.locator('.enter').click();
  stage=test.name+'_enter_workspace';await page.waitForURL('**/home');await page.locator('body[data-view=projects]').waitFor();
  assert.match(await page.locator('.buffer').innerText(),/Supa Agent/,'flagship_preserved');
  await page.screenshot({path:resolve(output,`${test.name}-home.png`)});
  const navigate=async label=>{if(test.isMobile)await page.locator('.tabline').click();await page.getByRole('option',{name:label,exact:true}).click();};
  const sections=[['Agents','agents',/No public agent profiles are available yet/],['Intelligence','intelligence',/No creator-approved reasoning reports/],['Memory','memory',/No approved public memory summaries/],['Economy','economy',/No verified public economic reports/],['Social','social',/No approved public activity/],['Trading · planned','trading',/No public trades/],['Token Launch · planned','token-launch',/No verified public token launches/],['Factory · planned','factory',/No public factory activity/],['About','about',/planned, not live/]];
  for(const [label,path,empty] of sections){
   stage=test.name+'_'+path;await navigate(label);await page.waitForURL('**/'+path);
   assert.match(await page.locator('.buffer').innerText(),empty);
   if(path!=='about'){
    const controls=page.locator('.buffer button');assert.ok(await controls.count()>0,'visible_disabled_action');
    for(const control of await controls.all())assert.equal(await control.isDisabled(),true,'unfinished_actions_disabled');
   }
   assert.equal(await page.locator('.buffer form, .buffer input, .buffer select').count(),0,'no_public_management_forms');
   await page.screenshot({path:resolve(output,`${test.name}-${path}.png`)});
   // Fresh direct navigation must select the same editorial view without auth.
   assert.equal((await page.goto(browserOrigin+'/'+path)).status(),200,'anonymous_deep_link');
   await page.locator('html[data-ready=true]').waitFor();assert.match(await page.locator('.buffer').innerText(),empty);
   assert.ok(!((await page.content()).includes(sentinel)),'no_private_fixture_on_any_section');
  }
  assert.equal(await page.getByRole('link',{name:'Official X / @supa_agency'}).getAttribute('href'),'https://x.com/supa_agency');
  await navigate('Agents');await page.waitForURL('**/agents');await navigate('About');await page.waitForURL('**/about');
  await page.goBack();await page.waitForURL('**/agents');await page.goForward();await page.waitForURL('**/about');
  if(test.isMobile){await page.locator('.tabline').click();assert.equal(await page.locator('.pane').evaluate(x=>x.inert),true);await page.keyboard.press('Escape');assert.equal(await page.locator('.tabline').getAttribute('aria-expanded'),'false');}
  else{await page.locator('.wheel').focus();await page.keyboard.press('ArrowUp');await page.waitForURL('**/factory');await page.keyboard.press('ArrowDown');await page.waitForURL('**/about');}
  await page.emulateMedia({reducedMotion:'no-preference'});await page.waitForFunction(()=>document.documentElement.dataset.motion==='running');await page.emulateMedia({reducedMotion:'reduce'});
  await page.evaluate(()=>document.fonts.ready);
  const layout=await page.evaluate(()=>({width:innerWidth,scrollWidth:document.documentElement.scrollWidth,logo:[...document.querySelectorAll('.brand img')].every(x=>x.complete&&x.naturalWidth>0),fonts:document.fonts.status}));
  assert.ok(layout.scrollWidth<=layout.width+2&&layout.logo&&layout.fonts==='loaded','responsive_logo_fonts');
  assert.equal(await page.locator('.wheel-item').count(),10);assert.ok(!((await page.content()).includes(sentinel)),'no_private_fixture');
  assert.equal(apiPolls,0,'no_public_api_polling');assert.equal(mutations,0,'no_public_writes');assert.equal(backendReads,before,'public_never_read_backend');
  report.contexts.push({name:test.name,fresh_private_context:true,enter_without_password:true,all_ten_public_sections:true,disabled_actions_verified:true,deep_links_and_history:true,...layout,api_polls:apiPolls,mutations,backend_reads:backendReads-before});await context.close();
 }
 for(const account of [visitor,...accounts]){
  stage='role_'+(account.role??'visitor');
  const headers={authorization:basic(account)};const page=await get('/agents',headers);assert.equal(page.status,200);assert.ok(!page.body.includes('data-public-landing="true"'),'authenticated_shell_preserved');
  const status=await get('/api/operator-status',headers);assert.equal(status.status,account.role?200:401,'role_status');
  if(account.role){assert.equal(JSON.parse(status.body).role,account.role);assert.equal(JSON.parse(status.body).eligibility.grant_owner,false);assert.equal((await get('/api/agents/11111111-1111-4111-8111-111111111111/memory',headers)).status,403);}
  const cross=await get('/api/operator-status',{...headers,origin:'https://untrusted.invalid'});assert.equal(cross.status,403);
  const context=await browser.newContext({reducedMotion:'reduce',serviceWorkers:'block',extraHTTPHeaders:headers});
  await context.route('**/*',route=>{const q=route.request();if(!['GET','HEAD'].includes(q.method())||new URL(q.url()).origin!==browserOrigin)return route.abort();return route.continue();});
  const signed=await context.newPage();signed.on('pageerror',e=>report.page_errors.push(e.name));await signed.goto(browserOrigin+'/agents');await signed.locator('html[data-ready=true]').waitFor();
  assert.equal(await signed.locator('.wheel-item').count(),10,'authenticated_navigation_preserved');
  assert.equal(await signed.locator('html').getAttribute('data-public-landing'),null,'authenticated_browser_not_public');
  await signed.waitForFunction(value=>document.querySelector('.buffer').textContent.includes(value),sentinel);await context.close();
  report.roles.push({role:account.role??'authenticated_visitor',private_status:status.status,cross_origin_status:cross.status,no_owner_escalation:true,authenticated_browser_passed:true});
 }
 for(const path of ['/operator','/wallet','/wallets','/management','/admin','/api/wallet','/api/agents','/api/operator-status','/agents/11111111-1111-4111-8111-111111111111','/memory/private','/token-launch/create','/economy/wallet','/factory/create','/social/drafts','/intelligence/control','/trading/orders'])assert.equal((await get(path)).status,401,'anonymous_private_denied');
 assert.equal((await get('/api/agents',{authorization:basic({username:'unauthorized',password:secret()})})).status,401);
 assert.equal(backendWrites,0);assert.deepEqual(report.page_errors,[]);report.backend_writes=backendWrites;report.passed=true;
}catch(error){report.failure=error instanceof assert.AssertionError?String(error.message).split('\n')[0].slice(0,120):'browser_rehearsal_failed';report.failure_stage=stage;report.error_type=error.name;process.exitCode=1;}
finally{
 await writeFile(resolve(output,'result.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
 await browser?.close().catch(()=>{});child?.kill('SIGTERM');next.kill('SIGTERM');backend.closeAllConnections();hostAdapter.closeAllConnections();await Promise.all([new Promise(r=>backend.close(r)),new Promise(r=>hostAdapter.close(r))]);
 if(profile){await new Promise(r=>setTimeout(r,300));await rm(profile,{recursive:true,force:true});}
}
