import { expect, test, type Page } from "@playwright/test";
import { agentId, records } from "./fixtures";

const runtimeErrors = new WeakMap<Page, string[]>();
test.beforeEach(({ page }) => {
  const errors: string[] = []; runtimeErrors.set(page, errors);
  page.on("pageerror", error => errors.push(error.name));
});
test.afterEach(({ page }) => { expect(runtimeErrors.get(page)).toEqual([]); });

test("original survival diagram responds to all four stages", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Born to think.");
  for (const name of ["Reason", "Authorize", "Record", "Observe"]) {
    const button = page.getByRole("button", { name: new RegExp(name) });
    await button.click(); await expect(button).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator(".diagram figcaption")).toContainText(name.toUpperCase());
  }
  await expect(page.locator(".diagram figcaption")).toContainText("does not run an agent");
});

test("navigation covers every page with no horizontal overflow", async ({ page, isMobile }) => {
  await page.goto("/");
  for (const [href, heading] of [["/agents", "Agent directory."], ["/economy", "Survival, accounted for."], ["/memory", "Memory & reasoning."], ["/social", "A voice. With receipts."], ["/trading", "Conviction needs limits."], ["/launch", "An idea is not a token."], ["/factory", "Intelligence, eventually²."]] as const) {
    if (isMobile) await page.getByRole("button", { name: "MENU +" }).click();
    await page.locator(`nav a[href='${href}']`).click();
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(heading);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  }
});

test("disconnected and empty states never invent agent data", async ({ page }) => {
  await page.goto("/agents");
  await expect(page.getByRole("heading", { name: "Console disconnected" })).toBeVisible();
  await page.route("**/api/console/agents", route => route.fulfill({ json: [] }));
  await page.getByRole("button", { name: "CHECK CONNECTION" }).click();
  await expect(page.getByRole("heading", { name: "No agents yet" })).toBeVisible();
  await expect(page.locator(".agent-card")).toHaveCount(0);
});

test("loading, sanitized error and recovery states are explicit", async ({ page }) => {
  let release: () => void = () => {};
  const gate = new Promise<void>(resolve => { release = resolve; });
  await page.route("**/api/console/agents", async route => { await gate; await route.fulfill({ status: 502, json: { error: "backend_contract_invalid" } }); });
  await page.goto("/agents");
  await expect(page.getByRole("heading", { name: "Reading the record…" })).toBeVisible();
  release();
  await expect(page.getByRole("heading", { name: "This record is unavailable" })).toBeVisible();
  await expect(page.getByText("backend_contract_invalid", { exact: true })).toBeVisible();
});

test.describe("fixture API components (synthetic, not Devnet evidence)", () => {
  test.beforeEach(async ({ page }) => {
    await page.route("**/api/console/**", async route => {
      const key = new URL(route.request().url()).pathname.split("/api/console/")[1];
      await route.fulfill({ status: key in records ? 200 : 404, json: key in records ? records[key] : { error: "resource_not_found" } });
    });
  });
  test("directory search and profile preserve lifecycle, permissions and goal estimates", async ({ page }) => {
    await page.goto("/agents"); await page.getByRole("searchbox").fill("absent");
    await expect(page.getByText("No matching agents")).toBeVisible();
    await page.getByRole("searchbox").fill("Fixture"); await page.locator(".agent-card").click();
    await expect(page).toHaveURL(new RegExp(`/agents/${agentId}`));
    await expect(page.getByText("PAUSED", { exact: true })).toBeVisible();
    await expect(page.getByText("25% model estimate")).toBeVisible();
    await expect(page.getByText("Capability boundary")).toBeVisible();
    await page.getByText("Event references", { exact: true }).click();
    await expect(page.locator("details[open]")).toContainText("CYCLE /");
  });
  test("economy separates simulation, mock Devnet and unavailable balances", async ({ page }) => {
    await page.goto("/economy");
    await expect(page.getByText("SIMULATED BALANCE", { exact: true })).toBeVisible();
    await expect(page.getByText("$2.5", { exact: true })).toBeVisible();
    await expect(page.getByText("Mock RPC evidence", { exact: false })).toBeVisible();
    await expect(page.getByText("No balance observation", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "table", exact: true }).click();
    await expect(page.getByRole("table")).toContainText("−0.001");
    await page.getByRole("button", { name: "chart", exact: true }).click();
    await expect(page.getByRole("img", { name: /recorded simulated expense/ })).toBeVisible();
    await expect(page.getByText("No verified receipts")).toBeVisible();
  });
  test("memory filters distinguish hypotheses and verified observations", async ({ page }) => {
    await page.goto("/memory");
    await expect(page.locator(".memory-card")).toHaveCount(2);
    await page.getByLabel("EVIDENCE TYPE").selectOption("hypothesis");
    await expect(page.locator(".memory-card")).toHaveCount(1);
    await page.getByText("Inspect provenance / 1 events").click();
    await expect(page.locator(".memory-card details[open]")).toContainText("CYCLE /");
    await page.getByLabel("SEARCH MEMORIES").fill("missing");
    await expect(page.getByText("No matching memories")).toBeVisible();
    await expect(page.getByText("Authorization / allowed")).toBeVisible();
  });
  test("social renders exact draft text separately from provenance and rejection codes", async ({ page }) => {
    await page.goto("/social");
    await expect(page.locator(".draft-text")).toHaveText("TEST FIXTURE: My simulated budget has boundaries. So does my enthusiasm for invoices.");
    await expect(page.getByText("Supa chose not to post.")).toBeVisible();
    await expect(page.getByText("Rejected fixture text must not be shown", { exact: false })).toHaveCount(0);
    await page.getByRole("combobox", { name: "Filter social decisions" }).selectOption("draft");
    await expect(page.locator(".draft-card")).toHaveCount(1);
    await page.getByText("Evidence / 1 events · 1 memories").click();
    await expect(page.locator(".draft-card details[open]")).toContainText("MEMORY /");
  });
});

for (const route of ["trading", "launch", "factory"]) {
  test(`${route} has no executable controls or mutation requests`, async ({ page }) => {
    const mutations: string[] = [];
    page.on("request", request => { if (!["GET", "HEAD"].includes(request.method())) mutations.push(request.method()); });
    await page.goto(`/${route}`);
    await expect(page.getByText("NON-EXECUTABLE PLACEHOLDER.")).toBeVisible();
    for (const input of await page.locator("main input, main select, main textarea, main button").all()) await expect(input).toBeDisabled();
    expect(mutations).toEqual([]);
  });
}

test("pixel typography, dark surfaces and accent colors satisfy the design", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => document.fonts.ready);
  const result = await page.evaluate(() => ({
    font: getComputedStyle(document.body).fontFamily,
    loaded: document.fonts.check('21px "VT323"'),
    background: getComputedStyle(document.body).backgroundColor,
    colors: [...document.querySelectorAll("body *")].flatMap(e => { const s = getComputedStyle(e); return [s.color, s.backgroundColor, s.borderTopColor]; }),
  }));
  expect(result.font).toContain("VT323"); expect(result.loaded).toBe(true); expect(result.background).toBe("rgb(11, 13, 12)");
  expect(result.colors.filter(c => { const n = c.match(/\d+/g)?.map(Number); return n && n[2] > n[0] + 12 && n[2] > n[1] + 5; })).toEqual([]);
});

test("smallest supported layout remains usable at 320 pixels", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 740 }); await page.goto("/");
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
});
