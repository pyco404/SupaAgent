import {test,expect} from '../../web/node_modules/@playwright/test/index.mjs';
import {navigation} from '../platform.js';
async function ready(page){await expect(page.locator('html')).toHaveAttribute('data-ready','true');}
async function home(page){await page.goto('/');await ready(page);await page.locator('.enter').click();await expect(page.locator('body')).toHaveAttribute('data-view','projects');}
async function command(page,value){await page.locator('.command-toggle').click();await page.locator('.command-input').fill(`supa ${value}`);await page.keyboard.press('Enter');}

test('all ten clean routes support direct entry refresh and matching selected cards',async({page})=>{
 for(const [index,item] of navigation.entries()){
  await page.goto(item.path+'?preview=public');await ready(page);
  if(item.id==='home')await page.locator('.enter').click();
  await expect(page.locator('.wheel-item[aria-selected=true]')).toHaveAttribute('data-index',String(index));
  await expect(page.locator('.tab-file')).toHaveText(item.label.split(' · ')[0]);
  expect(new URL(page.url()).pathname).toBe(item.path);
  await page.reload();await ready(page);
  await expect(page.locator('body')).toHaveAttribute('data-view','projects');
  await expect(page.locator('.wheel-item[aria-selected=true]')).toHaveAttribute('data-index',String(index));
  await expect(page.locator('link[rel=canonical]')).toHaveAttribute('href',item.path);
  expect(new URL(page.url()).search).toBe('?preview=public');
 }
});

test('legacy fragments migrate once without losing queries or creating a Back loop',async({page})=>{
 await page.goto('/about');await ready(page);
 for(const item of navigation){
  await page.goto(`/site/?preview=public#${encodeURIComponent(item.id)}`);await ready(page);
  await expect(page).toHaveURL(new RegExp(`${item.path.replaceAll('/','\\/')}\\?preview=public$`));
  await expect(page.locator('.wheel-item[aria-selected=true]')).toHaveAttribute('data-index',String(navigation.indexOf(item)));
 }
 await page.goto('/about');await ready(page);await page.goto('/site#memory');await ready(page);
 await expect(page).toHaveURL(/\/memory$/);await page.goBack();await expect(page).toHaveURL(/\/about$/);
 await page.goto('/site/#not-a-section');await expect(page.locator('h1')).toHaveText('404 / Page not found');
});

test('wheel terminal and Back Forward restore path selection without duplicate history',async({page})=>{
 await home(page);await command(page,'memory');await expect(page).toHaveURL(/\/memory$/);
 if(page.viewportSize().width<=700)await page.locator('button.tabline').click();
 await page.locator('.wheel').focus();await page.keyboard.press('ArrowDown');await expect(page).toHaveURL(/\/economy$/);
 await page.goBack();await expect(page).toHaveURL(/\/memory$/);await expect(page.locator('.tab-file')).toHaveText('Memory');
 await page.goForward();await expect(page).toHaveURL(/\/economy$/);await expect(page.locator('.tab-file')).toHaveText('Economy');
 await page.goBack();await page.goBack();await expect(page).toHaveURL(/\/$/);await expect(page.locator('body')).toHaveAttribute('data-view','projects');
 await page.goBack();await expect(page.locator('body')).toHaveAttribute('data-view','intro');
});

test('canonical paths preserve query state and unknown routes return genuine 404s',async({page,request})=>{
 const alias=await request.get('/memory/?preview=public',{maxRedirects:0});expect(alias.status()).toBe(308);expect(alias.headers().location).toBe('/memory?preview=public');
 for(const path of ['/missing','/memory/nested','/site/memory']){const response=await request.get(path);expect(response.status()).toBe(404);}
 await page.goto('/missing');await expect(page.getByRole('link',{name:'Return home'})).toHaveAttribute('href','/');
 await page.goto('/memory?preview=public');await ready(page);await command(page,'social');await expect(page).toHaveURL(/\/social\?preview=public$/);
 expect((await request.get('/api/not-a-resource')).status()).toBe(403);
 const auth=await request.get('/operator',{maxRedirects:0});expect(auth.status()).toBe(401);
 for(const path of ['/app.js','/style.css','/public/assets/supa-agent-icon.svg','/healthz'])expect((await request.get(path)).status()).toBe(200);
});
