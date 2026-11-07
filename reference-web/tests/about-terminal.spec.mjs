import {test,expect} from '../../web/node_modules/@playwright/test/index.mjs';
import {mkdir,writeFile} from 'node:fs/promises';
import {publicNavigation} from '../platform.js';
const evidence=new URL('../../artifacts/about-terminal/',import.meta.url);

test.afterEach(async({page})=>{await page.unrouteAll({behavior:'wait'});});
async function preview(page){
 const calls=[],errors=[];page.on('pageerror',e=>errors.push(e.name));
 await page.route('**/*',async route=>{
  const q=route.request(),u=new URL(q.url());
  if(u.origin!=='http://127.0.0.1:3201'||!['GET','HEAD'].includes(q.method())||u.pathname.startsWith('/api/')){calls.push(u.pathname);return route.abort();}
  if(['/',...publicNavigation.map(p=>p.path)].includes(u.pathname)){
   const r=await route.fetch({url:'http://127.0.0.1:3201'+(u.pathname==='/home'?'/':u.pathname)});
   return route.fulfill({response:r,body:(await r.text()).replace('data-motion="running"','data-motion="running" data-public-landing="true"')});
  }return route.continue();
 });return{calls,errors};
}
async function ready(page){await page.goto('/about');await expect(page.locator('html')).toHaveAttribute('data-ready','true');}

test('About architecture is readable, full-width, illustrative and keyboard inspectable',async({page},info)=>{
 const observed=await preview(page);await ready(page);
 const panel=page.getByRole('region',{name:'Terminal architecture'});
 await expect(panel).toBeVisible();await expect(page.locator('.blueprint')).toBeHidden();
 await expect(panel).toContainText('Illustrative / not executable');
 await expect(panel.locator('code')).toHaveCount(12);
 for(const term of ['canonical_id','creator_reviewed','private_by_default','owner_approved','explicit_grants','bounded','approval.required','execution.off'])await expect(panel).toContainText(term);
 await expect(panel.locator('button,input,form,canvas,img')).toHaveCount(0);
 await page.evaluate(()=>document.fonts.ready);
 const metrics=await panel.evaluate(el=>{
  const b=el.getBoundingClientRect(),buffer=document.querySelector('.buffer'),s=getComputedStyle(buffer);
  const lum=color=>{const c=color.match(/[\d.]+/g).slice(0,3).map(Number).map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4);return c[0]*.2126+c[1]*.7152+c[2]*.0722;};
  const bg=lum(getComputedStyle(document.body).backgroundColor);
  return{width:b.width,height:b.height,contentWidth:buffer.clientWidth-parseFloat(s.paddingLeft)-parseFloat(s.paddingRight),overflow:document.documentElement.scrollWidth>innerWidth,
   paints:performance.getEntriesByType('paint').map(p=>({name:p.name,ms:Math.round(p.startTime)})),
   minimumContrast:Math.min(...[...el.querySelectorAll('code,code span,.architecture-header span,summary')].map(e=>{const fg=lum(getComputedStyle(e).color);return(Math.max(bg,fg)+.05)/(Math.min(bg,fg)+.05);}))};
 });
 expect(metrics.width).toBeCloseTo(metrics.contentWidth,0);expect(metrics.height).toBeLessThanOrEqual(380);expect(metrics.overflow).toBe(false);expect(metrics.minimumContrast).toBeGreaterThanOrEqual(4.5);
 await mkdir(evidence,{recursive:true});await page.screenshot({path:new URL(`after-${info.project.name}.png`,evidence).pathname});
 const summary=panel.locator('summary');await summary.focus();await page.keyboard.press('Enter');await expect(panel.locator('details')).toHaveAttribute('open','');
 await expect(panel).toContainText('This page grants no approval and executes nothing.');await page.keyboard.press('Space');await expect(panel.locator('details')).not.toHaveAttribute('open','');
 await page.locator('.buffer').evaluate(el=>el.scrollTop=el.scrollHeight);
 const positions=await page.evaluate(()=>({buffer:document.querySelector('.buffer').getBoundingClientRect().bottom,footer:document.querySelector('.bar').getBoundingClientRect().top,last:document.querySelector('.buffer').lastElementChild.getBoundingClientRect().bottom}));
 expect(positions.buffer).toBeLessThanOrEqual(positions.footer-15);expect(positions.last).toBeLessThanOrEqual(positions.buffer);
 expect(observed.calls).toEqual([]);expect(observed.errors).toEqual([]);
 await writeFile(new URL(`${info.project.name}-metrics.json`,evidence),JSON.stringify(metrics,null,2));
});

test('code reveal and cursor respect reduced motion without running agent operations',async({page})=>{
 const observed=await preview(page);await page.emulateMedia({reducedMotion:'no-preference'});await ready(page);
 await expect(page.locator('.buffer')).toHaveAttribute('data-typing-state','typing');await page.locator('.skip-typing').click();
 await expect(page.locator('.architecture-code li').first()).toBeVisible();
 expect(await page.locator('.architecture-cursor').evaluate(el=>getComputedStyle(el).animationName)).toBe('terminal-caret-blink');
 await page.waitForTimeout(650);
 expect(await page.locator('.architecture-code').evaluateAll(nodes=>nodes.flatMap(n=>n.getAnimations({subtree:true})).filter(a=>a.playState==='running').length)).toBe(0);
 await page.emulateMedia({reducedMotion:'reduce'});
 expect(await page.locator('.architecture-cursor').evaluate(el=>getComputedStyle(el).animationName)).toBe('none');
 expect(await page.locator('.architecture-code li').first().evaluate(el=>getComputedStyle(el).animationName)).toBe('none');
 expect(observed.calls).toEqual([]);expect(observed.errors).toEqual([]);
});

test('composition belongs only to About and preserves protected routes',async({page,request})=>{
 const observed=await preview(page);await ready(page);
 for(const id of ['home','agents','factory','about']){
  await page.locator('.command-toggle').click();await page.locator('.command-input').fill('supa '+id);await page.keyboard.press('Enter');
  await expect(page).toHaveURL(new RegExp('/'+id+'$'));await expect(page.locator('.terminal-architecture')).toHaveCount(id==='about'?1:0);
  if(id==='home')await expect(page.locator('.blueprint')).toBeVisible();
 }
 for(const path of ['/operator','/my-agents','/api/operator-status','/api/agents/11111111-1111-4111-8111-111111111111/memory'])expect((await request.get(path)).status()).toBe(401);
 expect(observed.calls).toEqual([]);expect(observed.errors).toEqual([]);
});

test('narrow screens keep architecture and expanded details within the terminal scroll area',async({page})=>{
 await page.setViewportSize({width:320,height:640});const observed=await preview(page);await ready(page);
 await page.locator('.architecture-details summary').click();await page.locator('.architecture-details p').scrollIntoViewIfNeeded();
 expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
 await expect(page.locator('.workspace-hands')).toBeVisible();
 expect(observed.calls).toEqual([]);expect(observed.errors).toEqual([]);
});
