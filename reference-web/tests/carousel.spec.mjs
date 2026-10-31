import { test, expect } from '../../web/node_modules/@playwright/test/index.mjs';

async function openWheel(page, path = '/site/') {
  await page.goto(path);
  await expect(page.locator('html')).toHaveAttribute('data-ready', 'true');
  if (page.viewportSize().width <= 700) await page.locator('button.tabline').click();
}
const selected = page => page.locator('.wheel-item[aria-selected="true"]');

test('carousel entrance makes one sweep and settles on Home without changing sections', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await openWheel(page);
  await expect(page.locator('.wheel')).toHaveAttribute('data-carousel-state', 'entering');
  const card = page.locator('.wheel-item[data-index="0"]');
  const before = await card.evaluate(node => node.style.transform);
  await page.waitForTimeout(120);
  expect(await card.evaluate(node => node.style.transform)).not.toBe(before);
  await expect(selected(page)).toHaveAttribute('data-index', '0');
  await expect(page.locator('.wheel')).toHaveAttribute('data-carousel-state', 'settled');
  const settled = await card.evaluate(node => node.style.transform);
  await page.waitForTimeout(200);
  expect(await card.evaluate(node => node.style.transform)).toBe(settled);
  await expect(page).toHaveURL(/\/$/);
});

test('keyboard interrupts entrance and carousel wraps in both directions', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await openWheel(page);
  await page.locator('.wheel').focus();
  await page.keyboard.press('ArrowUp');
  await expect(page.locator('.wheel')).toHaveAttribute('data-carousel-state', 'settled');
  await expect(selected(page)).toHaveAttribute('data-index', '9');
  await page.keyboard.press('ArrowDown');
  await expect(selected(page)).toHaveAttribute('data-index', '0');
});

test('reduced motion skips sweep and explicit deep links remain available', async ({ page }) => {
  await openWheel(page, '/memory');
  await expect(page.locator('.wheel')).toHaveAttribute('data-carousel-state', 'settled');
  await expect(selected(page)).toHaveAttribute('data-index', '3');
});

test('entering again defaults to Home after a different section', async ({ page }) => {
  await openWheel(page, '/trading');
  if (page.viewportSize().width <= 700) await page.keyboard.press('Escape');
  await page.locator('.brand').click();
  await page.locator('.enter').click();
  await expect(page.locator('body')).toHaveAttribute('data-view', 'projects');
  await expect(selected(page)).toHaveAttribute('data-index', '0');
  await expect(page).toHaveURL(/\/$/);
});

test('drag interrupts sweep and selects only the released card', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await openWheel(page);
  const wheel = page.locator('.wheel');
  const box = await wheel.boundingBox();
  // Exercise the same pointer capture path with real touch events on mobile.
  if (page.viewportSize().width <= 700) {
    const cdp = await page.context().newCDPSession(page);
    const x = box.x + box.width / 2, y = box.y + box.height / 2;
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x, y }] });
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x, y: y - 85 }] });
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    await cdp.detach();
  } else {
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2 - 85, { steps: 5 });
    await page.mouse.up();
  }
  await expect(wheel).toHaveAttribute('data-carousel-state', 'settled');
  await expect(selected(page)).toHaveAttribute('data-index', '1');
  await expect(page).toHaveURL(/\/agents$/);
});

test('changing motion preference during sweep settles immediately', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await openWheel(page);
  await expect(page.locator('.wheel')).toHaveAttribute('data-carousel-state', 'entering');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await expect(page.locator('.wheel')).toHaveAttribute('data-carousel-state', 'settled');
  await expect(selected(page)).toHaveAttribute('data-index', '0');
  const transform = await selected(page).evaluate(node => node.style.transform);
  await page.waitForTimeout(150);
  expect(await selected(page).evaluate(node => node.style.transform)).toBe(transform);
});
