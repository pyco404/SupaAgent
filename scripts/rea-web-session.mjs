#!/usr/bin/env node
// An authenticated local browser for REA's explicit CDP connect capability.
// No credential is printed or passed in browser URLs, arguments or REA actions.
import { chromium } from "../web/node_modules/playwright-core/index.mjs";
import { readFileSync, lstatSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const envPath = path.join(root, "web/.env.local");
const stat = lstatSync(envPath);
if (!stat.isFile() || stat.isSymbolicLink() || (stat.mode & 0o077)) throw new Error("private_configuration_required");
const env = Object.fromEntries(readFileSync(envPath, "utf8").split("\n").filter(line => line && !line.startsWith("#")).map(line => { const offset = line.indexOf("="); return [line.slice(0, offset), line.slice(offset + 1)]; }));
if (!env.SUPA_WEB_USERNAME || !env.SUPA_WEB_PASSWORD) throw new Error("operator_authentication_required");
const width = Number(process.argv[2] ?? 1440), height = Number(process.argv[3] ?? 1000);
if (process.argv.length > 4 || !Number.isInteger(width) || !Number.isInteger(height) || width < 320 || width > 2560 || height < 600 || height > 1800) throw new Error("invalid_viewport");
const authorization = `Basic ${Buffer.from(`${env.SUPA_WEB_USERNAME}:${env.SUPA_WEB_PASSWORD}`).toString("base64")}`;
let ready = false;
for (let attempt = 0; attempt < 30; attempt++) {
  try { ready = (await fetch("http://127.0.0.1:3000/api/console/health", { headers: { authorization }, signal: AbortSignal.timeout(1000) })).ok; } catch { /* Only bounded local GET readiness checks. */ }
  if (ready) break;
  await new Promise(resolve => setTimeout(resolve, 500));
}
if (!ready) throw new Error("local_api_not_ready");
const context = await chromium.launchPersistentContext(path.join(root, "web/.browser-profile"), {
  executablePath: "/usr/bin/google-chrome", headless: true,
  args: ["--remote-debugging-port=9222", "--remote-debugging-address=127.0.0.1"],
  httpCredentials: { username: env.SUPA_WEB_USERNAME, password: env.SUPA_WEB_PASSWORD },
  viewport: { width, height }, colorScheme: "dark", reducedMotion: "reduce",
});
const page = context.pages()[0] ?? await context.newPage();
await page.goto("http://127.0.0.1:3000", { waitUntil: "networkidle" });
await page.getByText("API CONNECTED", { exact: true }).waitFor();
console.log(JSON.stringify({ cdp_endpoint: "http://127.0.0.1:9222", local_console: "http://127.0.0.1:3000", authenticated: true, credentials_printed: false }));
for (const event of ["SIGINT", "SIGTERM"]) process.on(event, async () => { await context.close(); process.exit(0); });
// The Playwright connection stays alive to answer authentication challenges for REA.
await new Promise(() => {});
