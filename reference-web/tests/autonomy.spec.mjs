import {test,expect} from '../../web/node_modules/@playwright/test/index.mjs';
const connected='http://127.0.0.1:3202';
const policy={mode:'offline',max_cycles:3,simulated_seed_microusd:10000,simulated_expense_limit_microusd:3300,paid_requests:0,tools:['read_memory','remember','evaluate_goal']};
test.use({httpCredentials:{username:'fixture',password:'fixture-password-'.repeat(3)}});
async function visit(page,handler){await page.route('**/api/autonomy',route=>route.request().method()==='GET'?route.fulfill({json:policy}):handler(route));await page.goto(connected+'/operator');await page.goto(connected+'/site/#intelligence');await page.locator('.autonomy-panel summary').click();}
test('offline workflow requires explicit consent and keeps goal typing out of navigation',async({page})=>{
  const calls=[];await visit(page,async route=>{const data=route.request().postDataJSON();calls.push(data);await route.fulfill({json:{agent_id:data.request_id,mode:'offline',simulated:true,cycles:[data.request_id,'22222222-2222-4222-8222-222222222222','33333333-3333-4333-8333-333333333333'],lifecycle:'paused',paid_requests:0,simulated_seed_microusd:10000,simulated_expense_limit_microusd:3300}});});
  await page.getByLabel('Goal / operator-only').fill('Review j k goals and remember verified outcomes');
  await page.getByLabel('Goal / operator-only').pressSequentially(' jk');
  await page.getByRole('button',{name:'Run authorized offline workflow'}).click();expect(calls).toHaveLength(0);
  await expect(page.locator('.tab-file')).toHaveText('Intelligence');
  await page.getByRole('checkbox').check();await page.getByRole('button',{name:'Run authorized offline workflow'}).click();
  await expect(page.locator('.autonomy-panel [role=status]')).toContainText('3 completed offline cycles');expect(calls).toHaveLength(1);expect(Object.keys(calls[0]).sort()).toEqual(['authorize_offline','goal','request_id']);
  await expect(page.getByRole('button',{name:'Inspect saved run'})).toBeVisible();expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  await page.getByRole('button',{name:'Prepare another workflow'}).click();await expect(page.getByLabel('Goal / operator-only')).toHaveValue('');await expect(page.getByRole('checkbox')).not.toBeChecked();
  await page.getByRole('button',{name:'Run authorized offline workflow'}).click();expect(calls).toHaveLength(1);
});
test('uncertain result persists the request hint across reload without automatic replay',async({page})=>{
  let calls=0;await visit(page,route=>{calls++;return route.fulfill({status:503,json:{error:'workflow_outcome_unknown_inspect_saved_run'}});});
  await page.getByLabel('Goal / operator-only').fill('Check saved outcomes before another attempt');await page.getByRole('checkbox').check();await page.getByRole('button',{name:'Run authorized offline workflow'}).click();
  await expect(page.locator('.autonomy-panel [role=status]')).toContainText('No automatic retry');const id=await page.evaluate(()=>sessionStorage.getItem('supa.offline-request'));
  await page.reload();await page.locator('.autonomy-panel summary').click();await expect(page.locator('.autonomy-panel [role=status]')).toContainText(id);expect(calls).toBe(1);await expect(page.locator('.autonomy-form')).toBeHidden();
});
test('an empty agent directory still exposes the authorized offline entry point',async({page})=>{
  await page.route('**/api/agents',route=>route.fulfill({json:[]}));await visit(page,()=>{throw Error('No run authorized');});await expect(page.getByLabel('Goal / operator-only')).toBeVisible();await expect(page.locator('.buffer')).toContainText('Select an agent');
});
