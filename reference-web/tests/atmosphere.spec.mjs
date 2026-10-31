import {test,expect} from '../../web/node_modules/@playwright/test/index.mjs';
const id='11111111-1111-4111-8111-111111111111';
// Public directory responses are synthetic fixtures; no agent is started or changed.
test('entity reflects only saved lifecycle records and clears unavailable evidence',async({page})=>{
 let lifecycle='paused',available=true;
 await page.route('**/api/agents',route=>available?route.fulfill({json:[{id,name:'Atmosphere fixture',lifecycle,created_at:'2026-10-31T00:00:00Z'}]}):route.fulfill({status:503,json:{error:'disconnected'}}));
 await page.goto('/site/#home');await expect(page.locator('html')).toHaveAttribute('data-ready','true');
 const entity=page.locator('.entity');await expect(entity).toHaveAttribute('data-lifecycle','paused');await expect(entity).toHaveAttribute('data-lifecycle-source','saved-record');
 for(const next of ['active','suspended','terminated','unrecognized']){
  lifecycle=next;await page.reload();await expect(page.locator('html')).toHaveAttribute('data-ready','true');await expect(entity).toHaveAttribute('data-lifecycle',next==='unrecognized'?'unavailable':next);
 }
 available=false;await page.reload();await expect(page.locator('html')).toHaveAttribute('data-ready','true');await expect(entity).toHaveAttribute('data-lifecycle','unavailable');await expect(entity).toHaveAttribute('data-lifecycle-source','unavailable');
 expect(await entity.getAttribute('aria-hidden')).toBe('true');
});
test('atmosphere stays behind controls and is still under reduced motion',async({page})=>{
 await page.emulateMedia({reducedMotion:'reduce'});await page.goto('/');await expect(page.locator('html')).toHaveAttribute('data-ready','true');
 const before=await page.evaluate(()=>({pointer:getComputedStyle(document.body,'::before').pointerEvents,z:getComputedStyle(document.body,'::before').zIndex,background:getComputedStyle(document.body).backgroundColor,animation:getComputedStyle(document.querySelector('.intro-mascot')).animationName}));
 expect(before.pointer).toBe('none');expect(before.z).toBe('-1');expect(before.animation).toBe('none');expect(before.background).toBe('rgb(8, 9, 8)');
 await page.keyboard.press('Tab');await expect(page.locator('.skip')).toBeFocused();await page.keyboard.press('Tab');await expect(page.locator('.brand')).toBeFocused();
 await page.locator('.enter').click();await expect(page.locator('body')).toHaveAttribute('data-view','projects');await expect(page.locator('.transition')).not.toHaveClass(/active/);
 const canvas=page.locator('.entity'),first=await canvas.evaluate(c=>c.toDataURL());await page.waitForTimeout(250);expect(await canvas.evaluate(c=>c.toDataURL())).toBe(first);
 expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
 const geometry=await page.locator('.wheel').boundingBox();if(geometry){await page.locator('.wheel').focus();await page.keyboard.press('ArrowDown');expect(await page.locator('.wheel-item[aria-selected=true]').count()).toBe(1);}
});
