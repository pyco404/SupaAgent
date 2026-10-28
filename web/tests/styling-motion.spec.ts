import { expect, test } from "@playwright/test";
import { agent, agentId, financials, records, timestamp } from "./fixtures";

async function fixtures(page: import("@playwright/test").Page) {
  await page.route("**/api/console/**", route => {
    const key = new URL(route.request().url()).pathname.split("/api/console/")[1];
    return route.fulfill({ status: key in records ? 200 : 404, json: key in records ? records[key] : { error: "resource_not_found" } });
  });
}

test("exact palette, readable pixel type and contrast are shared across controls", async ({ page }) => {
  await fixtures(page); await page.goto("/agents"); await expect(page.locator(".agent-card")).toBeVisible();
  const styles = await page.evaluate(() => {
    const root = getComputedStyle(document.documentElement);
    return {
      tokens: Object.fromEntries(["bg", "surface", "border", "text", "muted", "mint"].map(k => [k, root.getPropertyValue(`--${k}`).trim()])),
      fonts: [...document.querySelectorAll("h1, p, input, select, button")].map(e => getComputedStyle(e).fontFamily),
      bodySize: getComputedStyle(document.body).fontSize,
    };
  });
  expect(styles.tokens).toEqual({ bg: "#090b0a", surface: "#141816", border: "#29342d", text: "#f1f3ef", muted: "#a4afa7", mint: "#23d6a0" });
  expect(styles.fonts.every(font => font.includes("VT323"))).toBe(true); expect(styles.bodySize).toBe("22px");
  const luminance = (hex: string) => hex.slice(1).match(/../g)!.map(c => parseInt(c, 16) / 255).map(c => c <= .04045 ? c / 12.92 : ((c + .055) / 1.055) ** 2.4).reduce((s, c, i) => s + c * [.2126, .7152, .0722][i], 0);
  for (const fg of [styles.tokens.text, styles.tokens.muted, styles.tokens.mint]) for (const bg of [styles.tokens.bg, styles.tokens.surface]) expect((luminance(fg) + .05) / (luminance(bg) + .05)).toBeGreaterThan(4.5);
});

test("headline stagger and diagram transitions explain a selection without implying live activity", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" }); await page.goto("/");
  const lines = page.locator(".headline-line"); await expect(lines).toHaveCount(2);
  expect(await lines.nth(0).evaluate(e => getComputedStyle(e).animationDelay)).toBe("0s");
  expect(await lines.nth(1).evaluate(e => getComputedStyle(e).animationDelay)).toBe("0.09s");
  await expect(page.locator(".diagram")).toHaveAttribute("data-diagram-mode", "explanation");
  await expect(page.locator(".connection-trace")).toHaveCount(0);
  await page.getByRole("button", { name: /Authorize/ }).click();
  await expect(page.locator(".connection-trace")).toHaveCount(1);
  expect(await page.locator(".connection-trace").evaluate(e => getComputedStyle(e).animationIterationCount)).toBe("1");
  await expect(page.locator(".diagram figcaption")).toContainText("does not run an agent");
  await expect(page.locator(".lifecycle-active")).toHaveCount(0);
});

test("offscreen sections reveal once, with the same layout footprint", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" }); await page.goto("/");
  const section = page.locator(".editorial-section");
  await expect(section).toHaveAttribute("data-reveal", "pending");
  const before = await section.evaluate(e => (e as HTMLElement).offsetHeight);
  await section.scrollIntoViewIfNeeded(); await expect(section).toHaveAttribute("data-reveal", "visible");
  expect(await section.evaluate(e => (e as HTMLElement).offsetHeight)).toBe(before);
  await page.evaluate(() => window.scrollTo({ top: 0, behavior: "instant" }));
  await expect(section).toHaveAttribute("data-reveal", "visible");
});

test("background refresh retains panels and animates only the new exact financial value", async ({ page }) => {
  await fixtures(page);
  let amount = 2500000;
  let release: (() => void) | undefined;
  let pending = false;
  await page.route(`**/api/console/agents/${agentId}`, async route => {
    if (pending) await new Promise<void>(resolve => { release = resolve; });
    await route.fulfill({ json: { agent, financials: { ...financials, balance_microusd: amount } } });
  });
  await page.emulateMedia({ reducedMotion: "no-preference" }); await page.goto("/economy");
  const value = page.locator(".exact-value").first(); await expect(value).toHaveText("$2.5");
  await value.evaluate(e => { e.setAttribute("data-original-node", "true"); });
  // Hold the native animation clock so slow CI cannot miss a brief real update.
  await page.evaluate(() => {
    const animate = Element.prototype.animate;
    Element.prototype.animate = function(...args) {
      const animation = animate.apply(this, args);
      if (this.matches('[data-original-node="true"]')) {
        animation.pause(); Object.assign(window, { supaTestAnimation: animation });
      }
      return animation;
    };
  });
  pending = true; amount = 1234567;
  await page.getByRole("button", { name: "RELOAD SAVED RECORDS" }).click();
  await expect(value).toHaveText("$2.5"); await expect(page.getByText("RELOADING SAVED RECORDS…")).toBeVisible();
  await expect.poll(() => !!release).toBe(true); release!(); pending = false;
  await expect(value).toHaveText("$1.234567"); await expect(value).toHaveAttribute("data-original-node", "true");
  await expect(page.getByRole("heading", { name: "Reading the record…" })).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => (window as Window & { supaTestAnimation?: Animation }).supaTestAnimation?.playState)).toBe("paused");
  const animation = await page.evaluateHandle(() => (window as Window & { supaTestAnimation?: Animation }).supaTestAnimation!);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect.poll(() => animation.evaluate(a => a.playState)).toBe("idle");
});

test("new activity is refreshed from persisted records and only active agents pulse", async ({ page }) => {
  await fixtures(page); let updated = false;
  await page.route(`**/api/console/agents/${agentId}/activity`, route => route.fulfill({ json: updated ? [
    { id: "55555555-5555-4555-8555-555555555555", cycle_id: null, kind: "test_verified_event", code: "fixture_only", simulated: true, created_at: timestamp },
    ...(records[`agents/${agentId}/activity`] as object[]),
  ] : records[`agents/${agentId}/activity`] }));
  await page.emulateMedia({ reducedMotion: "no-preference" }); await page.goto(`/agents/${agentId}`);
  await expect(page.locator(".activity-record")).toHaveCount(1); await expect(page.locator(".lifecycle-active")).toHaveCount(0);
  updated = true; await page.getByRole("button", { name: "RELOAD SAVED RECORDS" }).click();
  await expect(page.locator(".activity-record")).toHaveCount(2);
  await expect(page.getByText("test verified event", { exact: true })).toBeVisible();
  expect(await page.locator(".activity-record").first().evaluate(e => getComputedStyle(e).animationName)).toBe("record-enter");
  await page.route(`**/api/console/agents/${agentId}`, route => route.fulfill({ json: { agent: { ...agent, lifecycle: "active" }, financials } }));
  await page.getByRole("button", { name: "RELOAD SAVED RECORDS" }).click(); await expect(page.locator(".lifecycle-active")).toHaveCount(1);
});

test("reduced motion disables page, headline, reveal, chart and menu animations", async ({ page }) => {
  await fixtures(page); await page.emulateMedia({ reducedMotion: "reduce" }); await page.goto("/");
  expect(await page.locator(".headline-line").first().evaluate(e => getComputedStyle(e).animationName)).toBe("none");
  await expect(page.locator('[data-reveal="pending"]')).toHaveCount(0);
  await page.getByRole("button", { name: /Record/ }).click();
  expect(await page.locator(".connection-trace").evaluate(e => getComputedStyle(e).animationName)).toBe("none");
  await page.goto("/economy"); await expect(page.locator(".expense-bar")).toHaveCount(1);
  expect(await page.locator(".expense-bar").evaluate(e => getComputedStyle(e).animationName)).toBe("none");
  expect(await page.locator(".expense-bar title").textContent()).toContain("simulated $0.001");
});

test("all console pages fit narrow mobile widths, with scrolling contained inside tables", async ({ page }) => {
  // This sweep loads nine documents and waits for each to settle.
  test.setTimeout(90_000);
  await fixtures(page); await page.setViewportSize({ width: 320, height: 740 });
  for (const path of ["/", "/agents", `/agents/${agentId}`, "/economy", "/memory", "/social", "/trading", "/launch", "/factory"]) {
    await page.goto(path); await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await page.waitForLoadState("networkidle");
    expect(await page.evaluate(() => document.documentElement.scrollWidth), path).toBeLessThanOrEqual(320);
  }
  await page.goto("/economy"); await page.getByRole("button", { name: "table", exact: true }).click();
  await expect(page.getByRole("table")).toContainText("−0.001");
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320);
});

test("keyboard focus remains visible and disconnected services have no active indicators", async ({ page }) => {
  await page.goto("/agents"); await expect(page.getByRole("heading", { name: "Console disconnected" })).toBeVisible();
  await expect(page.locator(".connection-label .disconnected")).toBeAttached();
  await expect(page.locator(".connection-label .active, .connection-label .connected")).toHaveCount(0);
  await expect(page.locator(".lifecycle-active")).toHaveCount(0);
  await page.keyboard.press("Tab"); await expect(page.getByRole("link", { name: "Skip to content" })).toBeFocused();
  const outline = await page.getByRole("link", { name: "Skip to content" }).evaluate(e => ({ color: getComputedStyle(e).outlineColor, width: getComputedStyle(e).outlineWidth }));
  expect(outline.color).toBe("rgb(35, 214, 160)"); expect(outline.width).toBe("2px");
});
