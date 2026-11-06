import {test,expect} from '../../web/node_modules/@playwright/test/index.mjs';
import {mkdir} from 'node:fs/promises';

const local='http://127.0.0.1:3336',origin='https://supaagency.xyz';
const basic=(name)=>'Basic '+Buffer.from(`${name}:${(name==='visitor'?'factory-visitor-fixture-':'factory-fixture-only-').repeat(3)}`).toString('base64');

test('public Factory flow preserves nine-section navigation and never requests private records',async({page},info)=>{
 const privateRequests=[],writes=[],errors=[];
 page.on('pageerror',e=>errors.push(e.name));
 await page.route('**/*',async route=>{
  const req=route.request(),url=new URL(req.url());
  if(!['GET','HEAD'].includes(req.method())){writes.push(url.pathname);return route.abort();}
  if(url.pathname.startsWith('/api/')){privateRequests.push(url.pathname);return route.abort();}
  if(url.origin!==origin)return route.abort();
  // Real Next proxy + route handler; only transport is mapped to loopback.
  const response=await route.fetch({url:local+url.pathname+url.search,
   headers:{...req.headers(),host:'supaagency.xyz'},maxRedirects:0});
  await route.fulfill({response});
 });
 await page.goto(origin);await expect(page.locator('html')).toHaveAttribute('data-ready','true');
 await page.locator('.enter').click();await expect(page).toHaveURL(origin+'/home');
 await page.getByRole('button',{name:'Create Agent →',exact:true}).click();
 await expect(page).toHaveURL(origin+'/factory');
 await expect(page.locator('.terminal-heading')).toHaveText(['Factory / Create Agents','1. Create Agent','2. Configure','3. Purpose Qualification','4. Agent Token / Optional','5. My Agents']);
 await expect(page.locator('.buffer')).toContainText('Skip this step and keep your agent token-free.');
 await expect(page.locator('.buffer')).toContainText('One original confirmed mint binds permanently');
 await expect(page.locator('.buffer input,.buffer textarea,.buffer form')).toHaveCount(0);
 await expect(page.locator('.buffer button:disabled')).toHaveCount(4);
 await expect(page.getByRole('link',{name:'My Agents / sign-in required'})).toHaveAttribute('href','/my-agents');
 await expect(page.locator('.wheel-item')).toHaveCount(9);
 await expect(page.locator('.wheel-name')).toHaveText(['Home','Agents','Intelligence','Memory','Economy','Social','Trading','Factory','About']);
 await expect(page.locator('.wheel-name').filter({hasText:'Token Launch'})).toHaveCount(0);
 if(info.project.name==='mobile')await page.locator('.tabline').click();
 await page.locator('.wheel-item[data-index="1"]').click();
 await expect(page).toHaveURL(origin+'/agents');
 await expect(page.locator('.buffer')).toContainText('No public agent profiles are available yet.');
 await expect(page.locator('.buffer')).toContainText('Supa Agent remains our flagship personality');
 await page.getByRole('button',{name:'Create Agent →',exact:true}).click();
 await expect(page).toHaveURL(origin+'/factory');
 await page.goBack();await expect(page).toHaveURL(origin+'/agents');
 await page.goForward();await expect(page).toHaveURL(origin+'/factory');
 await mkdir(info.outputDir,{recursive:true});await page.screenshot({path:info.outputPath('factory.png')});
 await page.goto(origin+'/token-launch');await expect(page).toHaveURL(origin+'/factory');
 for(const path of ['/home','/agents','/intelligence','/memory','/economy','/social','/trading','/factory','/about']){
  await page.goto(origin+path);await expect(page.locator('html')).toHaveAttribute('data-ready','true');
  await expect(page.locator('.buffer')).toHaveAttribute('aria-busy','false');
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1)).toBe(true);
 }
 expect(privateRequests).toEqual([]);expect(writes).toEqual([]);expect(errors).toEqual([]);
});

test('My Agents is authenticated, contains no records and cannot execute lifecycle actions',async({request})=>{
 for(const [role,expected] of [['anonymous',401],['invalid',401],['visitor',401],['viewer',200],['operator',200]]){
  const headers={host:'supaagency.xyz',...(role==='anonymous'?{}:{authorization:basic(role)})};
  const response=await request.get(local+'/my-agents',{headers});expect(response.status()).toBe(expected);
  if(expected===200){
   const html=await response.text();expect(html).toContain('owner management Coming Soon');
   expect(html.match(/type="button" disabled/g)).toHaveLength(4);
   expect(html).not.toMatch(/<script|<form|<input|api_token|PRIVATE_STAGING_SENTINEL/);
   expect(response.headers()['cache-control']).toBe('private, no-store');
  }
 }
 for(const path of ['/operator','/api/agents','/factory/create','/my-agents/private'])
  expect((await request.get(local+path,{headers:{host:'supaagency.xyz'}})).status()).toBe(401);
 expect((await request.get(local+'/my-agents',{headers:{host:'supaagency.xyz','x-supa-public-landing':'1','x-supa-experience':'1'}})).status()).toBe(401);
 expect((await request.get(local+'/my-agents',{headers:{host:'supaagency.xyz',origin:'https://evil.invalid',authorization:basic('operator')}})).status()).toBe(403);
 for(const path of ['/factory','/my-agents'])
  expect((await request.post(local+path,{headers:{host:'supaagency.xyz'},data:{name:'not-created'}})).status()).toBe(401);
 expect((await request.get(local+'/factory',{headers:{host:'127.0.0.1:3336'}})).status()).toBe(401);
});
