import {test,expect} from '../../web/node_modules/@playwright/test/index.mjs';
import {writeFile} from 'node:fs/promises';
import {publicNavigation,publicSectionView,sectionArtwork} from '../platform.js';
test.afterEach(async({page})=>{await page.unrouteAll({behavior:'wait'});});

async function publicPreview(page){
 const calls=[],errors=[];page.on('pageerror',error=>errors.push(error.name));
 await page.route('**/*',async route=>{
  const request=route.request(),url=new URL(request.url());
  if(url.origin!=='http://127.0.0.1:3201'||!['GET','HEAD'].includes(request.method())||url.pathname.startsWith('/api/')){calls.push(url.pathname);return route.abort();}
  if(['/',...publicNavigation.map(x=>x.path)].includes(url.pathname)){
   const response=await route.fetch({url:'http://127.0.0.1:3201'+(url.pathname==='/home'?'/':url.pathname)});
   return route.fulfill({response,body:(await response.text()).replace('data-motion="running"','data-motion="running" data-public-landing="true"')});
  }
  return route.continue();
 });
 return {calls,errors};
}

test('original artwork preserves nine-section public navigation and readable terminal composition',async({page},info)=>{
 const observed=await publicPreview(page);await page.goto('/');
 await expect(page.locator('html')).toHaveAttribute('data-ready','true');
 await page.locator('.enter').click();await expect(page).toHaveURL(/\/home$/);

 for(const [index,item]of publicNavigation.entries()){
  if(page.viewportSize().width<=700)await page.locator('.tabline').click();
  await page.locator(`.wheel-item[data-index="${index}"]`).click();
  await expect(page).toHaveURL(new RegExp(item.path+'$'));
  await expect(page.locator('.buffer')).toHaveAttribute('data-typing-state','complete');
  const artworks=publicSectionView(item.id).filter(row=>row.kind==='artwork');
  await expect(page.locator('.section-image')).toHaveCount(artworks.length);
  for(const [n,artwork]of artworks.entries()){
   const image=page.locator('.section-image').nth(n);await image.scrollIntoViewIfNeeded();
   await expect(image).toHaveAttribute('alt',sectionArtwork[artwork.id].alt);
   await expect.poll(()=>image.evaluate(el=>el.complete&&el.naturalWidth>0)).toBe(true);
   await expect(page.locator('.section-art-title').nth(n)).toHaveText(sectionArtwork[artwork.id].title);
   await expect(page.locator('.section-illustration figcaption').nth(n)).toContainText('not live activity');
  }
  await expect(page.locator('.workspace-hands')).toBeVisible();
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  const type=await page.locator('.buffer').evaluate(el=>{const s=getComputedStyle(el);return{size:parseFloat(s.fontSize),leading:parseFloat(s.lineHeight)/parseFloat(s.fontSize)};});
  expect(type.size).toBeGreaterThanOrEqual(13);expect(type.leading).toBeGreaterThanOrEqual(1.6);
  await page.screenshot({path:info.outputPath(`${item.id}.png`)});
 }
 expect(observed.calls).toEqual([]);expect(observed.errors).toEqual([]);
 await expect(page.locator('.wheel-thumb')).toHaveCount(9);
 const thumbnails=await page.locator('img.wheel-thumb').evaluateAll(nodes=>nodes.map(el=>({src:el.getAttribute('src'),lazy:el.loading,alt:el.alt,width:el.width,height:el.height})));
 expect(new Set(thumbnails.map(x=>x.src)).size).toBe(9);expect(thumbnails.every(x=>x.lazy==='lazy'&&x.alt==='')).toBe(true);
 await page.goBack();await expect(page).toHaveURL(/\/factory$/);
 await page.goto('/memory');await expect(page.locator('.section-illustration')).toHaveCount(0);
 expect(observed.calls).toEqual([]);
});

test('artwork respects reduced motion and remains optional on private operator views',async({page})=>{
 await page.goto('/factory');await expect(page.locator('html')).toHaveAttribute('data-ready','true');
 await expect(page.locator('.section-illustration')).toHaveCount(0);
 await expect(page.locator('.section-composition')).not.toHaveClass(/has-artwork/);
 const observed=await publicPreview(page);await page.goto('/home');
 await expect(page.locator('.section-illustration')).toBeVisible();
 expect(await page.locator('.section-image').evaluate(el=>getComputedStyle(el).animationName)).toBe('none');
 const branches=page.locator('.blueprint');await expect(branches).toBeVisible();
 expect(await branches.evaluate(el=>getComputedStyle(el).position)).not.toBe('absolute');
 expect(observed.errors).toEqual([]);expect(observed.calls).toEqual([]);
});

test('artwork assets have local immutable identities and no private-route side effects',async({request})=>{
 for(const section of Object.keys(sectionArtwork))for(const extension of ['thumb.webp','480.webp','960.webp','960.avif']){
  const response=await request.get(`/public/assets/supa-${section}-${extension}`);expect(response.status()).toBe(200);expect(response.headers()['content-type']).toContain(extension.endsWith('avif')?'image/avif':'image/webp');
  expect((await response.body()).length).toBeLessThan(160_000);
 }
 for(const path of ['/operator','/my-agents','/api/operator-status','/api/agents/11111111-1111-4111-8111-111111111111/memory'])expect([401,403]).toContain((await request.get(path)).status());
 expect((await request.post('/api/factory/agents',{data:{name:'must-not-create'}})).status()).toBe(405);
});

test('keyboard access contrast and local image budget remain within the terminal design',async({page},info)=>{
 const observed=await publicPreview(page);await page.goto('/');
 await expect(page.locator('html')).toHaveAttribute('data-ready','true');
 await page.keyboard.press('Tab');await expect(page.locator('.skip')).toBeFocused();
 await page.keyboard.press('Enter');await expect(page.locator('#workspace')).toBeFocused();
 await page.goto('/about');await expect(page.locator('.buffer')).toBeFocused();
 if(page.viewportSize().width<=700)await page.locator('.tabline').click();
 await page.locator('.wheel').focus();await page.keyboard.press('ArrowUp');
 await expect(page.locator('.wheel-item[aria-selected=true] .wheel-name')).toHaveText('Factory');
 if(page.viewportSize().width<=700)await page.keyboard.press('Escape');
 const measured=await page.evaluate(()=>{
  const luminance=value=>{const rgb=value.match(/[\d.]+/g).slice(0,3).map(Number).map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4);return rgb[0]*.2126+rgb[1]*.7152+rgb[2]*.0722;};
  const bg=luminance(getComputedStyle(document.body).backgroundColor);
  const contrast=[...document.querySelectorAll('.buffer p,.terminal-heading,.section-illustration figcaption span')].map(el=>{const foreground=luminance(getComputedStyle(el).color);return(Math.max(bg,foreground)+.05)/(Math.min(bg,foreground)+.05);});
  return{minimumTextContrast:Math.min(...contrast),overflow:document.documentElement.scrollWidth>innerWidth,imagesWithoutAlt:document.querySelectorAll('img:not([alt])').length,paints:performance.getEntriesByType('paint').map(x=>({name:x.name,ms:Math.round(x.startTime)})),sectionImageBytes:performance.getEntriesByType('resource').filter(x=>/supa-[a-z]+-(480|960)\.(webp|avif)$/.test(x.name)).map(x=>({path:new URL(x.name).pathname,decodedBodyBytes:x.decodedBodySize}))};
 });
 expect(measured.minimumTextContrast).toBeGreaterThanOrEqual(4.5);expect(measured.overflow).toBe(false);expect(measured.imagesWithoutAlt).toBe(0);
 expect(observed.calls).toEqual([]);expect(observed.errors).toEqual([]);
 await writeFile(info.outputPath(`accessibility-performance.json`),JSON.stringify({...measured,scope:'local Chromium fixtures; not a WCAG or production performance certification',privateRequests:observed.calls.length},null,2));
});
