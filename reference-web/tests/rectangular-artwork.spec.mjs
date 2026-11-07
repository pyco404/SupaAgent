import {test,expect} from '../../web/node_modules/@playwright/test/index.mjs';
import {mkdir,writeFile} from 'node:fs/promises';
import {publicNavigation} from '../platform.js';

test.afterEach(async({page})=>{await page.unrouteAll({behavior:'wait'});});
async function publicPreview(page){
 const denied=[],errors=[];page.on('pageerror',error=>errors.push(error.name));
 await page.route('**/*',async route=>{
  const request=route.request(),url=new URL(request.url());
  if(url.origin!=='http://127.0.0.1:3201'||!['GET','HEAD'].includes(request.method())||url.pathname.startsWith('/api/')){denied.push(url.pathname);return route.abort();}
  if(['/',...publicNavigation.map(item=>item.path)].includes(url.pathname)){
   const response=await route.fetch({url:'http://127.0.0.1:3201'+(url.pathname==='/home'?'/':url.pathname)});
   return route.fulfill({response,body:(await response.text()).replace('data-motion="running"','data-motion="running" data-public-landing="true"')});
  }
  return route.continue();
 });
 return {denied,errors};
}
async function rectangularFrame(page){
 const image=page.locator('.section-image');
 await expect.poll(()=>image.evaluate(el=>el.complete&&el.naturalWidth>0)).toBe(true);
 const frame=await image.evaluate(el=>{
  const box=el.getBoundingClientRect(),style=getComputedStyle(el);
  return {width:box.width,height:box.height,fit:style.objectFit,naturalWidth:el.naturalWidth,naturalHeight:el.naturalHeight};
 });
 expect(frame.width/frame.height).toBeGreaterThanOrEqual(1.7);
 expect(frame.height).toBeGreaterThan(50);expect(frame.fit).toBe('cover');
 // Frame changes must not resample or stretch the approved square source assets.
 expect(frame.naturalWidth).toBe(frame.naturalHeight);
 expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
 return frame;
}

test('all nine sections show landscape artwork and rectangular navigation crops',async({page,request},info)=>{
 const observed=await publicPreview(page);await page.goto('/home');
 await expect(page.locator('html')).toHaveAttribute('data-ready','true');
 const frames=[];await mkdir('../artifacts/rectangular-artwork',{recursive:true});
 for(const [index,item]of publicNavigation.entries()){
  if(page.viewportSize().width<=700)await page.locator('.tabline').click();
  await page.locator(`.wheel-item[data-index="${index}"]`).click();
  await expect(page).toHaveURL(new RegExp(item.path+'$'));
  frames.push({section:item.id,...await rectangularFrame(page)});
  if(page.viewportSize().width<=700)await page.locator('.tabline').click();
  const thumbnail=await page.locator('.wheel-item[aria-selected=true] img').evaluate(el=>({width:el.clientWidth,height:el.clientHeight,fit:getComputedStyle(el).objectFit}));
  expect(thumbnail.width/thumbnail.height).toBeGreaterThan(1.25);expect(thumbnail.fit).toBe('cover');
  if(page.viewportSize().width<=700)await page.keyboard.press('Escape');
  await expect(page.locator('.buffer')).toHaveAttribute('data-typing-state','complete');
  await expect(page.locator('.buffer form,.buffer input')).toHaveCount(0);
  if(['home','factory','about'].includes(item.id))await page.screenshot({path:`../artifacts/rectangular-artwork/${info.project.name}-${item.id}.png`});
 }
 for(const path of ['/operator','/my-agents','/api/agents/11111111-1111-4111-8111-111111111111/memory'])expect((await request.get(path)).status()).toBe(401);
 expect(observed.denied).toEqual([]);expect(observed.errors).toEqual([]);
 await writeFile(`../artifacts/rectangular-artwork/${info.project.name}-frames.json`,JSON.stringify(frames,null,2));
});

test('short screens keep rectangular art clear of the terminal and footer',async({page})=>{
 await page.setViewportSize({width:844,height:390});const observed=await publicPreview(page);await page.goto('/factory');
 await expect(page.locator('html')).toHaveAttribute('data-ready','true');await rectangularFrame(page);
 const layout=await page.evaluate(()=>({buffer:document.querySelector('.buffer').getBoundingClientRect().bottom,footer:document.querySelector('.bar').getBoundingClientRect().top}));
 expect(layout.buffer).toBeLessThanOrEqual(layout.footer-15);
 expect(observed.denied).toEqual([]);expect(observed.errors).toEqual([]);
});
