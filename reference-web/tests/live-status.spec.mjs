import { test, expect } from '../../web/node_modules/@playwright/test/index.mjs';
const id='11111111-1111-4111-8111-111111111111';
test.use({httpCredentials:{username:'fixture',password:'fixture-password-'.repeat(3)}});
test('read-only polling refreshes changed goals without restarting unchanged text',async({page})=>{
  let description='Initial saved goal',reads=0;
  await page.clock.install();
  await page.route(`**/api/agents/${id}/goals`,async route=>{
    reads++;await route.fulfill({json:[{id,description,completed:false,priority:1,suggested_progress_bps:0}]});
  });
  await page.goto('http://127.0.0.1:3202/operator');
  await page.goto('http://127.0.0.1:3202/site/#intelligence');
  const buffer=page.locator('.buffer');
  await expect(buffer).toContainText(description);await expect(buffer).toHaveAttribute('data-typing-state','complete');
  const firstRow=page.locator('.buffer > :first-child');
  await firstRow.evaluate(el=>el.dataset.renderCanary='retained');
  await page.clock.fastForward(15100);await expect.poll(()=>reads).toBeGreaterThan(1);
  await expect(firstRow).toHaveAttribute('data-render-canary','retained');
  description='New verified saved goal';
  // Polling preserves keyboard focus while an operator reads a record.
  await buffer.focus();await page.clock.fastForward(15100);
  await expect.poll(()=>reads).toBeGreaterThan(2);await expect(buffer).not.toContainText(description);
  await page.locator('.brand').focus();await page.clock.fastForward(15100);
  await expect(buffer).toContainText(description);
  await expect(buffer).toHaveAttribute('data-typing-state','complete');
});
