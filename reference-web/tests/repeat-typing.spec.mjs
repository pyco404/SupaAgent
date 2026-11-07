import {test,expect} from '../../web/node_modules/@playwright/test/index.mjs';
import {publicNavigation} from '../platform.js';

test.afterEach(async({page})=>{await page.unrouteAll({behavior:'wait'});});
async function publicPreview(page){
 const denied=[];
 await page.route('**/*',async route=>{
  const request=route.request(),url=new URL(request.url());
  if(url.origin!=='http://127.0.0.1:3201'||!['GET','HEAD'].includes(request.method())||url.pathname.startsWith('/api/')){denied.push(url.pathname);return route.abort();}
  if(['/',...publicNavigation.map(item=>item.path)].includes(url.pathname)){
   const response=await route.fetch({url:'http://127.0.0.1:3201'+(url.pathname==='/home'?'/':url.pathname)});
   return route.fulfill({response,body:(await response.text()).replace('data-motion="running"','data-motion="running" data-public-landing="true"')});
  }
  return route.continue();
 });
 return denied;
}
async function choose(page,id){
 await page.locator('.command-toggle').click();await page.locator('.command-input').fill(`supa ${id}`);await page.keyboard.press('Enter');
 await expect(page).toHaveURL(new RegExp(`/${id}$`));
}
async function complete(page){
 const buffer=page.locator('.buffer');
 await expect(buffer).toHaveAttribute('data-typing-state','complete');
 await expect(buffer).toHaveAttribute('aria-busy','false');
 await expect(buffer).toHaveAttribute('aria-live','polite');
 await expect(buffer.locator('[hidden],[inert]')).toHaveCount(0);
 await expect(page.locator('.skip-typing')).toBeHidden();
}

test('first visits type; repeat navigation, history and refresh show complete text',async({page})=>{
 await page.emulateMedia({reducedMotion:'no-preference'});const denied=await publicPreview(page);
 await page.goto('/about');await expect(page.locator('.buffer')).toHaveAttribute('data-typing-state','typing');
 await page.locator('.skip-typing').click();await complete(page);
 await choose(page,'factory');await expect(page.locator('.buffer')).toHaveAttribute('data-typing-state','typing');
 // Leave while typing: the canceled timer must never overwrite the new page.
 await choose(page,'about');await complete(page);
 await page.goBack();await expect(page).toHaveURL(/\/factory$/);await complete(page);
 await page.goForward();await expect(page).toHaveURL(/\/about$/);await complete(page);
 await page.reload();await complete(page);await expect(page.locator('.buffer')).toContainText('Supa Agency');
 await page.waitForTimeout(350);await complete(page);
 const ids=await page.evaluate(()=>JSON.parse(sessionStorage.getItem('supa.typed-sections.v1.public')));
 expect(ids.sort()).toEqual(['about','factory']);expect(denied).toEqual([]);
});

test('blocked browser storage still permits instant repeat visits',async({page})=>{
 await page.addInitScript(()=>{Storage.prototype.getItem=()=>{throw new Error('Unavailable');};Storage.prototype.setItem=()=>{throw new Error('Unavailable');};});
 await page.emulateMedia({reducedMotion:'no-preference'});const denied=await publicPreview(page);
 await page.goto('/about');await expect(page.locator('.buffer')).toHaveAttribute('data-typing-state','typing');
 await choose(page,'factory');await expect(page.locator('.buffer')).toHaveAttribute('data-typing-state','typing');
 await choose(page,'about');await complete(page);expect(denied).toEqual([]);
});

test('reduced motion reveals new sections immediately and ignores malformed stored preferences',async({page})=>{
 await page.addInitScript(()=>sessionStorage.setItem('supa.typed-sections.v1.public','{"unexpected":"value"}'));
 const denied=await publicPreview(page);await page.goto('/about');await complete(page);
 await choose(page,'factory');await complete(page);expect(denied).toEqual([]);
});
