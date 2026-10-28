import { expect, test } from "@playwright/test";
import { agent, agentId, records } from "./fixtures";

async function fixtures(page: import("@playwright/test").Page) {
  await page.route("**/api/console/**", route => {
    const key = new URL(route.request().url()).pathname.split("/api/console/")[1];
    return route.fulfill({ status: key in records ? 200 : 404, json: key in records ? records[key] : { error: "resource_not_found" } });
  });
}

test("public introduction explains current inspection and planned execution", async ({ page }) => {
  await fixtures(page); await page.goto("/");
  await expect(page.locator(".hero")).toContainText("Solana-native platform");
  await expect(page.locator(".hero")).toContainText("read-only preview");
  await expect(page.locator(".hero")).toContainText("Planned");
  await page.getByRole("link", { name: /How agents work/i }).click();
  await expect(page.locator("#how-agents-work")).toBeInViewport();
});

test("planned destinations are disclosed before navigation", async ({ page, isMobile }) => {
  await page.goto("/");
  if (isMobile) await page.getByRole("button", { name: "MENU +" }).click();
  const nav = page.getByRole("navigation", { name: isMobile ? "Mobile navigation" : "Main navigation", exact: true });
  for (const name of ["Trading", "Token launch", "Factory"]) await expect(nav.getByRole("link", { name: new RegExp(name) })).toContainText("Planned");
});

test("directory uses saved evidence counts and omits personality instructions", async ({ page }) => {
  await fixtures(page); await page.goto("/agents");
  const card = page.locator(".agent-card");
  await expect(card).toContainText("1 recorded event"); await expect(card).toContainText("Devnet");
  await expect(card).not.toContainText(agent.personality.voice);
  await card.click(); await expect(page.getByRole("navigation", { name: "Agent records" })).toBeVisible();
  await page.getByText("Technical configuration", { exact: true }).click();
  await expect(page.locator("details[open]")).toContainText("Maximum tools");
  await expect(page.locator("main")).not.toContainText(agent.personality.voice);
});

test("agent identity follows contextual links and query selection", async ({ page }) => {
  await fixtures(page); await page.goto(`/agents/${agentId}`);
  await page.getByRole("navigation", { name: "Agent records" }).getByRole("link", { name: "Economy", exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`/economy\\?agent=${agentId}`));
  await expect(page.locator(".selected-identity")).toHaveText(agent.name);
  await expect(page.getByRole("combobox", { name: "Inspect agent" })).toHaveValue(agentId);
});

test("record retrieval is distinct from chain observation and formulas are disclosed", async ({ page }) => {
  await fixtures(page); await page.goto("/economy");
  await expect(page.getByRole("button", { name: /Reload saved records/i })).toBeEnabled();
  await expect(page.locator(".records-feedback time")).toHaveAttribute("datetime", /T/);
  await expect(page.getByText("Not enough recorded expenses to estimate runway.", { exact: true })).toBeVisible();
  await expect(page.getByText("Insufficient recorded simulated expense history.", { exact: true })).not.toBeVisible();
  await page.getByText("Calculation details", { exact: true }).click();
  await expect(page.locator("details[open]")).toContainText("Insufficient recorded simulated expense history.");
  await expect(page.getByText("Reloading saved records does not query Solana. A new chain observation requires an authorized operator.", { exact: true })).toBeVisible();
  await expect(page.getByText("Mainnet activity is unavailable in this preview.")).toBeVisible();
});

test("successful retrieval waits for all active resources and never claims success on partial error", async ({ page }) => {
  await fixtures(page); let release: (() => void) | undefined;
  await page.route(`**/api/console/agents/${agentId}/wallet/payments`, async route => {
    await new Promise<void>(resolve => { release = resolve; });
    await route.fulfill({ status: 503, json: { error: "backend_disconnected" } });
  });
  await page.goto("/economy"); await expect.poll(() => !!release).toBe(true);
  await expect(page.locator(".records-feedback time")).toHaveCount(0);
  release!(); await expect(page.locator(".records-feedback")).toContainText("Some saved records are unavailable");
  await expect(page.locator(".records-feedback time")).toHaveCount(0);
});

test("empty chart avoids meaningless view controls and context explains a created agent", async ({ page }) => {
  await fixtures(page);
  await page.route(`**/api/console/agents/${agentId}/simulated-ledger`, route => route.fulfill({ json: [] }));
  await page.goto("/economy"); await expect(page.getByText("No recorded expenses")).toBeVisible();
  await expect(page.getByRole("button", { name: "chart", exact: true })).toHaveCount(0);
  await page.route(`**/api/console/agents/${agentId}`, route => route.fulfill({ json: { ...(records[`agents/${agentId}`] as object), agent: { ...agent, lifecycle: "created" } } }));
  await page.goto(`/agents/${agentId}`); await expect(page.getByText(/Created means this agent has not been activated/)).toBeVisible();
});

test("mobile links have comfortable targets and long selected identities wrap", async ({ page }) => {
  await fixtures(page); await page.setViewportSize({ width: 320, height: 740 });
  await page.goto("/");
  for (const link of await page.locator(".text-link, .header-inner .brand").all()) expect((await link.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  const longName = "A very long persisted agent identity that must remain readable on a narrow screen";
  await page.route("**/api/console/agents", route => route.fulfill({ json: [{ ...agent, name: longName }] }));
  await page.route(`**/api/console/agents/${agentId}`, route => route.fulfill({ json: { ...(records[`agents/${agentId}`] as object), agent: { ...agent, name: longName } } }));
  await page.goto("/memory"); await expect(page.locator(".selected-identity")).toHaveText(longName);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320);
});


test("an explicit unavailable agent never silently falls back to another identity", async ({ page }) => {
  await fixtures(page);
  const requested: string[] = [];
  page.on("request", request => { if (request.url().includes("/api/console/")) requested.push(new URL(request.url()).pathname); });
  await page.goto("/memory?agent=66666666-6666-4666-8666-666666666666");
  await expect(page.getByRole("heading", { name: "This record is unavailable" })).toBeVisible();
  expect(requested).toContain("/api/console/agents/66666666-6666-4666-8666-666666666666");
  expect(requested).not.toContain(`/api/console/agents/${agentId}/memory`);
});


test("empty memory reaches its result without irrelevant search controls", async ({ page }) => {
  await fixtures(page); await page.setViewportSize({ width: 390, height: 844 });
  await page.route(`**/api/console/agents/${agentId}/memory`, route => route.fulfill({ json: [] }));
  await page.goto("/memory");
  const result = page.getByRole("heading", { name: "Nothing remembered yet" });
  await expect(result).toBeVisible();
  await expect(page.getByRole("searchbox")).toHaveCount(0);
  expect((await result.boundingBox())!.y).toBeLessThan(844);
});
