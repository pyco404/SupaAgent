import {test,expect} from '../../web/node_modules/@playwright/test/index.mjs';
import {publicNavigation} from '../platform.js';

const placement={home:['home'],agents:['agents'],intelligence:[],memory:[],economy:[],social:[],trading:[],factory:['factory','intelligence'],about:[]};
test.afterEach(async({page})=>{await page.unrouteAll({behavior:'wait'});});
async function publicPreview(page){
 const denied=[],errors=[],images=[];page.on('pageerror',error=>errors.push(error.name));
 page.on('request',request=>{if(/supa-[a-z]+-(480|960)\.(webp|avif)$/.test(new URL(request.url()).pathname))images.push(new URL(request.url()).pathname);});
 await page.route('**/*',async route=>{
  const request=route.request(),url=new URL(request.url());
  if(url.origin!=='http://127.0.0.1:3201'||!['GET','HEAD'].includes(request.method())||url.pathname.startsWith('/api/')){denied.push(url.pathname);return route.abort();}
  if(['/',...publicNavigation.map(item=>item.path)].includes(url.pathname)){
   const response=await route.fetch({url:'http://127.0.0.1:3201'+(url.pathname==='/home'?'/':url.pathname)});
   return route.fulfill({response,body:(await response.text()).replace('data-motion="running"','data-motion="running" data-public-landing="true"')});
  }
  return route.continue();
 });
 return {denied,errors,images};
}
async function frames(page){
 for(const figure of await page.locator('.section-illustration').all()){
  await figure.scrollIntoViewIfNeeded();const image=figure.locator('.section-image');
  await expect.poll(()=>image.evaluate(el=>el.complete&&el.naturalWidth>0)).toBe(true);
  const frame=await image.evaluate(el=>{
   const box=el.getBoundingClientRect(),style=getComputedStyle(el),buffer=document.querySelector('.buffer'),bs=getComputedStyle(buffer);
   return {width:box.width,height:box.height,content:buffer.clientWidth-parseFloat(bs.paddingLeft)-parseFloat(bs.paddingRight),fit:style.objectFit,naturalWidth:el.naturalWidth,naturalHeight:el.naturalHeight,loading:el.loading,reserved:el.width/el.height};
  });
  expect(frame.width/frame.height).toBeCloseTo(16/9,1);expect(frame.width).toBeCloseTo(frame.content,0);
  expect(frame.fit).toBe('cover');expect(frame.naturalWidth).toBe(frame.naturalHeight);
  expect(frame.loading).toBe('lazy');expect(frame.reserved).toBeCloseTo(16/9,1);
  expect(await figure.locator('figcaption').textContent()).toContain('Concept artwork / not live activity');
 }
}

test('editorial placement preserves all nine sections, full-width crops and navigation thumbnails',async({page,request})=>{
 const observed=await publicPreview(page);await page.goto('/home');await expect(page.locator('html')).toHaveAttribute('data-ready','true');
 for(const [index,item]of publicNavigation.entries()){
  if(page.viewportSize().width<=700)await page.locator('.tabline').click();
  await page.locator(`.wheel-item[data-index="${index}"]`).click();await expect(page).toHaveURL(new RegExp(item.path+'$'));
  await expect(page.locator('.buffer')).toHaveAttribute('data-typing-state','complete');
  expect(await page.locator('.buffer .section-illustration').evaluateAll(nodes=>nodes.map(el=>el.dataset.section))).toEqual(placement[item.id]);
  await expect(page.locator('.section-composition .section-image')).toHaveCount(0);
  await frames(page);
  // Pictures follow the content they explain; there is no automatic first-row hero.
  for(const image of await page.locator('.section-illustration').all())expect(await image.evaluate(el=>el.previousElementSibling?.className)).toBe('terminal-line');
  if(page.viewportSize().width<=700)await page.locator('.tabline').click();
  const thumbnail=await page.locator('.wheel-item[aria-selected=true] img').evaluate(el=>({width:el.clientWidth,height:el.clientHeight,fit:getComputedStyle(el).objectFit}));
  expect(thumbnail.width/thumbnail.height).toBeGreaterThan(1.25);expect(thumbnail.fit).toBe('cover');
  if(page.viewportSize().width<=700)await page.keyboard.press('Escape');
  await expect(page.locator('.buffer form,.buffer input')).toHaveCount(0);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
 }
 for(const path of ['/operator','/my-agents','/api/agents/11111111-1111-4111-8111-111111111111/memory'])expect((await request.get(path)).status()).toBe(401);
 expect(observed.denied).toEqual([]);expect(observed.errors).toEqual([]);
});

test('text-first deep links never request full-size illustrations',async({page})=>{
 const observed=await publicPreview(page);
 for(const path of ['intelligence','memory','economy','social','trading','about']){
  await page.goto('/'+path);await expect(page.locator('html')).toHaveAttribute('data-ready','true');
  await expect(page.locator('.section-image')).toHaveCount(0);
 }
 expect(observed.images).toEqual([]);expect(observed.denied).toEqual([]);expect(observed.errors).toEqual([]);
});

test('multiple images reserve their space before decode and remain clear of the footer on short screens',async({page})=>{
 await page.setViewportSize({width:844,height:390});const observed=await publicPreview(page);
 let release;const held=new Promise(resolve=>release=resolve);
 await page.route(/supa-(factory|intelligence)-960\.avif$/,async route=>{await held;return route.continue();});
 try{
  await page.goto('/factory',{waitUntil:'domcontentloaded'});await expect(page.locator('html')).toHaveAttribute('data-ready','true');
  const before=await page.locator('.section-image').evaluateAll(nodes=>nodes.map(el=>({width:el.getBoundingClientRect().width,height:el.getBoundingClientRect().height})));
  expect(before).toHaveLength(2);for(const b of before)expect(b.width/b.height).toBeCloseTo(16/9,1);
  release();await frames(page);
  const after=await page.locator('.section-image').evaluateAll(nodes=>nodes.map(el=>({width:el.getBoundingClientRect().width,height:el.getBoundingClientRect().height})));
  expect(after).toEqual(before);
  await page.locator('.buffer').evaluate(el=>el.scrollTop=el.scrollHeight);
  const layout=await page.evaluate(()=>({buffer:document.querySelector('.buffer').getBoundingClientRect().bottom,footer:document.querySelector('.bar').getBoundingClientRect().top,last:document.querySelector('.buffer').lastElementChild.getBoundingClientRect().bottom}));
  expect(layout.buffer).toBeLessThanOrEqual(layout.footer-15);expect(layout.last).toBeLessThanOrEqual(layout.buffer);
 }finally{release();}
 expect(observed.denied).toEqual([]);expect(observed.errors).toEqual([]);
});
