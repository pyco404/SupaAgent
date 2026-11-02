import {test,expect} from '../../web/node_modules/@playwright/test/index.mjs';
const origin='http://127.0.0.1:3202',first='11111111-1111-4111-8111-111111111111',second='22222222-2222-4222-8222-222222222222';
test.use({httpCredentials:{username:'fixture',password:'fixture-password-'.repeat(3)}});
async function launchFixture(page){
  let lifecycle='paused';
  await page.route('**/api/launch',route=>route.fulfill({json:{mode:'disposable_offline',enabled:true,role:'owner',expires_at:'2099-01-01T00:00:00Z',paid_requests_enabled:false}}));
  await page.route('**/api/agents',route=>route.fulfill({json:[{id:first,name:'MVP first',lifecycle,created_at:'2026-10-28T00:00:00Z'},{id:second,name:'MVP second',lifecycle:'paused',created_at:'2026-10-28T00:00:00Z'}]}));
  await page.goto(origin+'/agents');await expect(page.locator('.launch-panel')).toBeVisible();
  return ()=>{lifecycle='active';};
}
test('selected identity survives a full reload without authorizing anything',async({page})=>{
 await launchFixture(page);await page.locator('#agent-picker-trigger').click();await page.getByRole('option',{name:'MVP second / paused',exact:true}).click();await expect(page.locator('#agent-select')).toHaveValue(second);await page.reload();await expect(page.locator('#agent-select')).toHaveValue(second);
});
test('a blurred launch draft survives a background lifecycle update',async({page})=>{
 const change=await launchFixture(page);await page.locator('#launch-name').fill('Do not discard this draft');await page.locator('.buffer').focus();change();await page.waitForTimeout(16500);await expect(page.locator('#launch-name')).toHaveValue('Do not discard this draft');
});
test('definite launch rejection explains recovery without an automatic retry',async({page})=>{
 await launchFixture(page);let posts=0;await page.route('**/api/launch',async route=>{if(route.request().method()==='POST'){posts++;return route.fulfill({status:403,json:{error:'owner_required'}});}return route.fulfill({json:{mode:'disposable_offline',enabled:true,role:'owner',expires_at:'2099-01-01T00:00:00Z',paid_requests_enabled:false}});});
 await page.locator('#launch-name').fill('Permission test');await page.locator('#launch-goal').fill('Observe only supplied test records.');await page.locator('[name=launch-consent]').check();await page.locator('.launch-panel button[type=submit]').click();await expect(page.locator('.launch-panel [role=status]')).toContainText('Permission denied');expect(posts).toBe(1);
});
