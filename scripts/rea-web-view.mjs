#!/usr/bin/env node
// Prepare an actual measured viewport, then use REA's read-only inspection and
// capture_web_screenshot tools. Attached scenario settings alone can crop a
// caller-owned desktop window without proving responsive layout dimensions.
import { chromium } from "../web/node_modules/playwright-core/index.mjs";
import { readFileSync, lstatSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const pages = {
  home: ["/", ".overview-status .mint"], agents: ["/agents", ".agent-card"],
  profile: ["/agents/fab35ded-49c2-4168-b078-09a9470092c2", ".metric-grid"],
  economy: ["/economy", ".evidence-line"], memory: ["/memory", ".two-column > section:first-child .empty"],
  social: ["/social", ".empty"], trading: ["/trading", ".planned-summary"],
  launch: ["/launch", ".planned-summary"], factory: ["/factory", ".planned-summary"],
};
const name = process.argv[2] ?? "home", width = Number(process.argv[3] ?? 1440), height = Number(process.argv[4] ?? 1000);
if (!(name in pages) || process.argv.length > 5 || !Number.isInteger(width) || !Number.isInteger(height) || width < 320 || width > 2560 || height < 600 || height > 1800) throw new Error("invalid_local_view");
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const file = path.join(root, "web/.env.local"), stat = lstatSync(file);
if (!stat.isFile() || stat.isSymbolicLink() || (stat.mode & 0o077)) throw new Error("private_configuration_required");
const env = Object.fromEntries(readFileSync(file, "utf8").split("\n").filter(line => line && !line.startsWith("#")).map(line => { const i = line.indexOf("="); return [line.slice(0, i), line.slice(i + 1)]; }));
const browser = await chromium.connectOverCDP("http://127.0.0.1:9222");
try {
  const context = browser.contexts()[0], page = context.pages()[0];
  await context.setHTTPCredentials({ username: env.SUPA_WEB_USERNAME, password: env.SUPA_WEB_PASSWORD });
  await page.setViewportSize({ width, height });
  const response = await page.goto(`http://127.0.0.1:3000${pages[name][0]}`, { waitUntil: "domcontentloaded" });
  if (!response?.ok()) throw new Error("local_page_unavailable");
  await page.locator(pages[name][1]).waitFor();
  if (!["home", "trading", "launch", "factory"].includes(name)) {
    await page.waitForFunction(() => /Retrieved|Some saved records are unavailable/.test(document.querySelector(".records-feedback")?.textContent ?? ""));
  }
  await page.evaluate(() => document.fonts.ready);
  await page.evaluate(() => window.scrollTo(0, 0));
  const dimensions = await page.evaluate(() => ({ width: innerWidth, height: innerHeight, scroll_width: document.documentElement.scrollWidth }));
  if (dimensions.width !== width || dimensions.height !== height || dimensions.scroll_width > width) throw new Error("responsive_dimensions_failed");
  console.log(JSON.stringify({ page: name, path: pages[name][0], actual_viewport: dimensions, status: response.status(), synthetic_data: false }));
} finally { await browser.close(); }
