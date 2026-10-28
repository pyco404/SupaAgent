import { test, expect } from '../../web/node_modules/@playwright/test/index.mjs';
const id='11111111-1111-4111-8111-111111111111';
test.use({httpCredentials:{username:'fixture',password:'fixture-password-'.repeat(3)}});
test('read-only polling refreshes changed goals without restarting unchanged text',async({page})=>{
  test.setTimeout(75000);
  let description='Initial saved goal',reads=0;
  await page.route(`**/api/agents/${id}/goals`,async route=>{
    reads++;await route.fulfill({json:[{id,description,completed:false,priority:1,suggested_progress_bps:0}]});
  });
  await page.goto('http://127.0.0.1:3202/operator');
  await page.goto('http://127.0.0.1:3202/site/#intelligence');
  const buffer=page.locator('.buffer');
  await expect(buffer).toContainText(description);await expect(buffer).toHaveAttribute('data-typing-state','complete');
  await page.waitForLoadState('networkidle');
  const firstRow=page.locator('.buffer > :first-child');
  await firstRow.evaluate(el=>el.dataset.renderCanary='retained');
  // Use the real interval: jumping a mocked browser clock can expire live HTTP timeouts.
  let before=reads;await expect.poll(()=>reads,{timeout:20000}).toBeGreaterThan(before);
  await page.waitForLoadState('networkidle');await expect(firstRow).toHaveAttribute('data-render-canary','retained');
  description='New verified saved goal';
  await buffer.focus();before=reads;
  await expect.poll(()=>reads,{timeout:20000}).toBeGreaterThan(before);await page.waitForLoadState('networkidle');
  await expect(buffer).not.toContainText(description);
  await page.locator('.brand').focus();
  await expect(buffer).toContainText(description,{timeout:20000});
  await expect(buffer).toHaveAttribute('data-typing-state','complete');
});
