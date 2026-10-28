import { test, expect } from "../../web/node_modules/@playwright/test/index.mjs";

const navigationFocus = page => page.locator(page.viewportSize().width <= 700 ? "button.tabline" : ".wheel");
async function open(page, path) {
  await page.goto(path);
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
}

test("intro enters the project browser and browser back restores intro", async ({ page }) => {
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await open(page, "/");
  await expect(page.locator("body")).toHaveAttribute("data-view", "intro");
  await expect(page.locator("a.enter")).toHaveAttribute("href", "/site/");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/site\/(?:#.*)?$/);
  await expect(page.locator("body")).toHaveAttribute("data-view", "projects");
  await expect(page.locator(".bar")).toBeVisible();
  await page.goBack();
  await expect(page.locator("body")).toHaveAttribute("data-view", "intro");
  expect(errors).toEqual([]);
});

test("project browser supports keyboard selection and Escape focus", async ({ page }) => {
  await open(page, "/site/");
  const options = page.getByRole("option", { includeHidden: true });
  await expect(options.first()).toBeAttached();
  const selected = () => page.locator('.wheel-item[aria-selected="true"]');
  const initial = await selected().getAttribute("data-index");
  if (page.viewportSize().width <= 700) await page.locator("button.tabline").click();
  await page.locator(".wheel").focus();
  await page.keyboard.press("ArrowDown");
  await expect.poll(() => selected().getAttribute("data-index")).not.toBe(initial);
  await page.keyboard.press("k");
  await expect(selected()).toHaveAttribute("data-index", initial);
  await page.keyboard.press("Escape");
  await expect(navigationFocus(page)).toBeFocused();
});

test("terminal scrolling keys preserve the selected project", async ({ page }) => {
  await open(page, "/site/");
  const selected = page.locator('.wheel-item[aria-selected="true"]');
  const initial = await selected.getAttribute("data-index");
  await page.locator(".buffer").focus();
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("j");
  await expect(selected).toHaveAttribute("data-index", initial);
  await expect(page.locator(".buffer")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(navigationFocus(page)).toBeFocused();
});

test("command input is keyboard accessible and does not navigate externally", async ({ page }) => {
  await open(page, "/site/");
  await expect(page.locator(".wheel-item").first()).toBeAttached();
  await page.locator(".buffer").focus();
  await page.keyboard.press(":");
  await expect(page.locator(".command-input")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(navigationFocus(page)).toBeFocused();
  await expect(page).toHaveURL(/\/site\/(?:#.*)?$/);
  await page.keyboard.press(":");
  await page.locator(".command-input").fill("globalThis.__supaCommandExecuted=true");
  await page.keyboard.press("Enter");
  await expect(page.locator(".announcement")).toContainText("Unknown command");
  expect(await page.evaluate(() => globalThis.__supaCommandExecuted)).toBeUndefined();
});

test("project hashes restore direct and nested project selections", async ({ page }) => {
  await open(page, "/site/#waybar-themes");
  await expect(page.locator('.wheel-item[aria-selected="true"]')).toHaveAttribute("data-index", "3");
  await expect(page.locator('.wheel-item[aria-selected="true"]')).toContainText("waybar-themes");
  await page.locator(".buffer").focus();
  await page.keyboard.press(":");
  await page.locator(".command-input").fill("e GitHub-Traffic-Board");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/site\/#GitHub-Traffic-Board$/i);
  await expect(page.locator('.wheel-item[aria-selected="true"]')).toHaveAttribute("aria-label", "more/");
  await expect(page.locator(".buffer")).toContainText("open GitHub-Traffic-Board");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
  await expect(page.locator('.wheel-item[aria-selected="true"]')).toHaveAttribute("aria-label", "more/");
  await expect(page.locator(".buffer")).toContainText("open GitHub-Traffic-Board");
});

test("dark palette survives reload and live motion preference changes are respected", async ({ page }) => {
  await open(page, "/");
  const background = () => page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--bg").trim());
  const original = await background();
  await page.locator(".theme-key").click();
  await expect.poll(background).not.toBe(original);
  const chosen = await background();
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
  expect(await background()).toBe(chosen);
  const rgb = await page.locator("body").evaluate(element => getComputedStyle(element).backgroundColor);
  expect(rgb.match(/[\d.]+/g).slice(0, 3).every(value => Number(value) < 60)).toBe(true);
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await expect(page.locator("html")).toHaveAttribute("data-motion", "running");
  // Commit a rendered motion frame before changing the emulated preference again.
  // Chromium can coalesce back-to-back emulation changes in the same paint cycle.
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.locator("html")).toHaveAttribute("data-motion", "still");
});

test("normal terminal boot restores complete accessible text when motion is reduced", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/site/", { waitUntil: "domcontentloaded" });
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
  const buffer = page.locator(".buffer");
  await expect(buffer).toHaveAttribute("aria-busy", "true");
  await expect(buffer).toHaveAttribute("aria-live", "off");
  await expect(page.locator(".entity-state b").nth(1)).toHaveText("typing");
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(buffer).toHaveAttribute("aria-busy", "false");
  await expect(buffer).toHaveAttribute("aria-live", "polite");
  await expect(buffer.getByRole("link", { name: "github", exact: true })).toBeVisible();
  expect(await buffer.locator("[hidden]").count()).toBe(0);
  await expect(page.locator(".entity-state b").nth(1)).toHaveText("idle");
});

test("changing projects cancels the previous terminal boot without restoring stale text", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/site/", { waitUntil: "domcontentloaded" });
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
  await expect(page.locator(".buffer")).toHaveAttribute("aria-busy", "true");
  await page.keyboard.press("Escape");
  await expect(navigationFocus(page)).toBeFocused();
  if (page.viewportSize().width <= 700) await page.locator("button.tabline").click();
  await page.locator(".wheel").focus();
  await page.keyboard.press("ArrowDown");
  await expect(page.locator('.wheel-item[aria-selected="true"]')).toHaveAttribute("data-index", "1");
  await expect(page.locator(".buffer")).toHaveAttribute("aria-busy", "true");
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.locator(".buffer")).toHaveAttribute("aria-busy", "false");
  await expect(page.locator(".buffer")).toContainText("open Shibumi-Shell");
  await expect(page.locator(".buffer")).not.toContainText("fetch HANCORE-linux");
  await expect(page.locator(".buffer").getByRole("link", { name: "view repository ↗" })).toBeVisible();
  await expect(page.locator(".entity-state b").nth(1)).toHaveText("idle");
});

test("mobile drawer toggles and Escape restores its trigger focus", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await open(page, "/site/");
  await expect(page.locator(".wheel-item").first()).toBeAttached();
  const toggle = page.locator("button.tabline");
  await expect(toggle).toBeVisible();
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await page.keyboard.press("Escape");
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(page.locator("button.tabline")).toBeFocused();
});

test("project wheel responds to pointer scrolling and card selection", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await open(page, "/site/");
  await expect(page.locator(".wheel-item").first()).toBeAttached();
  const selected = () => page.locator('.wheel-item[aria-selected="true"]');
  const initial = await selected().getAttribute("data-index");
  await page.locator(".wheel").hover();
  await page.mouse.wheel(0, 100);
  await expect.poll(() => selected().getAttribute("data-index")).not.toBe(initial);
  const target = page.locator(`.wheel-item[data-index="${initial}"]`);
  await target.click();
  await expect(selected()).toHaveAttribute("data-index", initial);
});

test("reference preview has no backend, analytics or paid API requests", async ({ page }) => {
  const requests = [];
  const errors = [];
  page.on("request", request => requests.push(request.url()));
  page.on("pageerror", error => errors.push(error.message));
  await open(page, "/");
  await page.locator("a.enter").click();
  await expect(page.locator("body")).toHaveAttribute("data-view", "projects");
  await expect(page.locator(".bar")).toBeVisible();
  for (const url of requests) expect(["http://127.0.0.1:3201", "https://hancore.dev"]).toContain(new URL(url).origin);
  expect(requests.some(url => /(?:api\.openai|solana|\/api\/|analytics|plausible)/i.test(url))).toBe(false);
  expect(errors).toEqual([]);
});

test("responsive scenes fit all supported widths with reduced motion", async ({ page }) => {
  for (const width of [320, 390, 768, 1440]) {
    await page.setViewportSize({ width, height: width < 500 ? 844 : 1000 });
    for (const route of ["/", "/site/"]) {
      await open(page, route);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      await expect(page.locator("html")).toHaveAttribute("data-motion", "still");
    }
  }
});

test("preview is read only and cannot expose project secrets or parent paths", async ({ request }) => {
  const health = await request.get("/healthz");
  expect(health.status()).toBe(200);
  expect(await health.json()).toMatchObject({ mode: "isolated-reference", host: "127.0.0.1" });
  expect(health.headers()["content-security-policy"]).toContain("connect-src 'self'");
  expect(health.headers()["content-security-policy"]).toContain("frame-ancestors 'none'");
  for (const path of ["/.env", "/.env.eval", "/server.mjs", "/package.json", "/public/.env", "/%2e%2e/.env.eval", "/public/%2e%2e/%2e%2e/.env.eval", "/%2f%2e%2e%2f.env.eval"]) {
    const response = await request.get(path);
    expect([400, 404]).toContain(response.status());
    expect(await response.text()).not.toMatch(/sk-[A-Za-z0-9]|OPENAI_API_KEY|postgres(ql)?:\/\//);
  }
  expect((await request.post("/", { data: "mutation" })).status()).toBe(405);
});
