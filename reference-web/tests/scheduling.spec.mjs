import{test,expect}from'../../web/node_modules/@playwright/test/index.mjs';
const id='11111111-1111-4111-8111-111111111111';
const policy={goal_ids:[id],interval_seconds:5,max_cycles:3,simulated_budget_microusd:3300,valid_for_seconds:3600};
function status(lifecycle='created'){return {agent_id:id,lifecycle,revision:lifecycle==='created'?0:2,enabled:lifecycle==='active',policy:lifecycle==='created'?null:policy,cycles_started:0,reserved_microusd:0,expires_at:lifecycle==='created'?null:'2026-10-29T00:00:00Z',next_cycle_at:'2026-10-28T23:00:00Z',last_error:null,mode:'offline',simulated:true,paid_requests_enabled:false};}
test.use({httpCredentials:{username:'fixture',password:'fixture-password-'.repeat(3)}});
async function visit(page,handler,view=status()){
  await page.route('**/api/agents/*/schedule',route=>route.request().method()==='GET'?route.fulfill({json:view}):handler(route));
  await page.route('**/api/agents/*/goals',route=>route.fulfill({json:[{id,description:'Review verified outcomes',completed:false,priority:1,suggested_progress_bps:0}]}));
  await page.goto('http://127.0.0.1:3202/operator');await page.goto('http://127.0.0.1:3202/site/#intelligence');await page.locator('.schedule-panel summary').click();
}
test('scheduling requires goal selection and explicit consent without triggering navigation',async({page})=>{
  const calls=[];await visit(page,async route=>{calls.push(route.request().postDataJSON());await route.fulfill({json:status('active')});});
  const panel=page.locator('.schedule-panel');await panel.getByLabel('Operator reason').fill('Review outcomes j k');await panel.getByLabel('Operator reason').pressSequentially(' jk');
  await panel.getByRole('button',{name:'Apply authorized lifecycle operation'}).click();expect(calls).toHaveLength(0);
  await panel.locator('[name=schedule-authorization]').check();await panel.getByRole('button',{name:'Apply authorized lifecycle operation'}).click();await expect(panel.getByRole('status')).toContainText('Select at least one');
  await panel.locator('[name=approved-goal]').check();await expect(panel.locator('[name=schedule-authorization]')).not.toBeChecked();await panel.locator('[name=schedule-authorization]').check();await panel.getByRole('button',{name:'Apply authorized lifecycle operation'}).click();await expect(panel.getByRole('status')).toContainText('Saved start');expect(calls).toHaveLength(1);
  expect(calls[0].policy.goal_ids).toEqual([id]);expect(calls[0].expected_revision).toBe(0);expect(calls[0].authorize_offline).toBe(true);await expect(panel.getByRole('button',{name:'Apply authorized lifecycle operation'})).toBeDisabled();expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
});
test('pause and permanent stop require fresh consent and uncertain responses are never replayed',async({page})=>{
  let calls=0;await visit(page,async route=>{calls++;await route.fulfill({status:503,json:{error:'unknown'}});},status('active'));
  const panel=page.locator('.schedule-panel');await expect(panel.getByLabel('Lifecycle operation')).toHaveValue('pause');await panel.getByLabel('Operator reason').fill('Pause for review');await panel.locator('[name=schedule-authorization]').check();await panel.getByLabel('Lifecycle operation').selectOption('stop');await expect(panel.locator('[name=schedule-authorization]')).not.toBeChecked();
  await panel.locator('[name=schedule-authorization]').check();await panel.getByRole('button',{name:'Apply authorized lifecycle operation'}).click();await expect(panel.getByRole('status')).toContainText('No automatic retry');expect(calls).toBe(1);
  await page.reload();await page.locator('.schedule-panel summary').click();expect(calls).toBe(1);
});
test('terminated identities expose saved history and no executable controls',async({page})=>{
  await visit(page,()=>{throw Error('No operation authorized');},status('terminated'));const panel=page.locator('.schedule-panel');await expect(panel.locator('.schedule-form')).toBeHidden();await expect(panel).toContainText('permanently terminated');await expect(panel.getByRole('button',{name:'Reload saved schedule'})).toBeVisible();
});

test('owner renewal requires specific new limits and clears consent when they change',async({page})=>{
  const calls=[];await visit(page,async route=>{calls.push(route.request().postDataJSON());await route.fulfill({json:status('active')});},status('paused'));
  const panel=page.locator('.schedule-panel');await panel.getByLabel('Lifecycle operation').selectOption('renew');await expect(panel.locator('[name=approved-goal]')).not.toBeChecked();
  await panel.locator('[name=approved-goal]').check();await panel.getByLabel('Operator reason').fill('Fresh finite renewal approval');await panel.locator('[name=schedule-authorization]').check();
  await panel.locator('[name=max_cycles]').fill('2');await expect(panel.locator('[name=schedule-authorization]')).not.toBeChecked();
  await panel.locator('[name=schedule-authorization]').check();await panel.getByRole('button',{name:'Apply authorized lifecycle operation'}).click();await expect(panel.getByRole('status')).toContainText('Saved renew');
  expect(calls).toHaveLength(1);expect(calls[0].policy.max_cycles).toBe(2);expect(calls[0].policy.goal_ids).toEqual([id]);expect(Number.isFinite(Date.parse(calls[0].consented_at))).toBe(true);
});
test('viewer reads genuine health contract without lifecycle controls or inferred running state',async({page})=>{
  await page.route('**/api/agents/*/access',route=>route.fulfill({json:{role:'viewer'}}));
  await page.route('**/api/agents/*/execution-health',route=>route.fulfill({json:{agent_id:id,status:'delayed',execution_running:false,worker_enabled:true,lifecycle:'active',checked_at:'2026-10-28T23:00:00Z',heartbeat_at:'2026-10-28T22:59:59Z',tick_finished_at:null,lease_until:null,cycle_id:null}}));
  await visit(page,()=>{throw Error('No write permitted');},status('active'));
  await expect(page.locator('.schedule-panel form')).toBeHidden();await expect(page.locator('.buffer')).toContainText('Your role  viewer');await expect(page.locator('.buffer')).toContainText('Worker health  delayed');await expect(page.locator('.buffer')).toContainText('No verified running execution');
});
