import { test, expect } from '../../web/node_modules/@playwright/test/index.mjs';

// Check the rendered color, including ancestor opacity: changing only the token
// would miss the original status-label regression.
test('visible status labels meet normal-text contrast in every approved palette', async ({ page }) => {
  for (let palette = 0; palette < 3; palette++) {
    await page.goto('/');
    await expect(page.locator('html')).toHaveAttribute('data-ready', 'true');
    await page.locator('.theme-key').click();
    await page.goto('/site/', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('html')).toHaveAttribute('data-ready', 'true');
    const samples = await page.locator('.bar .stat > span').evaluateAll(labels => {
      const rgb = value => value.match(/[\d.]+/g).slice(0, 3).map(Number);
      const luminance = color => color.map(v => {
        v /= 255;
        return v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4;
      }).reduce((sum, v, i) => sum + v * [.2126, .7152, .0722][i], 0);
      return labels.filter(el => el.getClientRects().length).map(el => {
        let opacity = 1;
        for (let ancestor = el; ancestor; ancestor = ancestor.parentElement) {
          opacity *= Number(getComputedStyle(ancestor).opacity);
        }
        const background = rgb(getComputedStyle(document.body).backgroundColor);
        const foreground = rgb(getComputedStyle(el).color).map((v, i) => v * opacity + background[i] * (1 - opacity));
        const a = luminance(foreground), b = luminance(background);
        return { label: el.textContent, ratio: (Math.max(a, b) + .05) / (Math.min(a, b) + .05) };
      });
    });
    if (page.viewportSize().width <= 700) expect(samples).toEqual([]);
    else expect(samples.length).toBeGreaterThan(0);
    for (const sample of samples) expect(sample.ratio, `${sample.label}, palette ${palette}`).toBeGreaterThanOrEqual(4.5);
    if (page.viewportSize().width > 1100) await expect(page.locator('.entity-state').first()).toBeVisible();
  }
});
