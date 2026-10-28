import { expect, test } from "@playwright/test";
import { agentId } from "./fixtures";

const protectedOrigin = "http://127.0.0.1:3101";
const authorization = `Basic ${Buffer.from(`fixture:${"local-test-only-".repeat(3)}`).toString("base64")}`;

test("configured console requires operator authentication", async ({ request }) => {
  for (const path of ["/", "/agents", "/api/console/agents"]) {
    const response = await request.get(`${protectedOrigin}${path}`);
    expect(response.status()).toBe(401); expect(response.headers()["cache-control"]).toBe("no-store");
  }
});
test("server-only bearer and unknown backend fields never reach the browser", async ({ request }) => {
  const response = await request.get(`${protectedOrigin}/api/console/health`, { headers: { authorization } });
  expect(response.status()).toBe(200); expect(response.headers()["cache-control"]).toBe("no-store");
  expect(await response.json()).toEqual({ status: "ok", economic_mode: "simulation", publishing_enabled: false });
  expect(await response.text()).not.toContain("fixture-token");
});
test("all implemented read resources satisfy the backend contract", async ({ request }) => {
  for (const path of ["agents", `agents/${agentId}`, ...["memory", "activity", "cycles", "actions", "social", "drafts", "simulated-ledger", "wallet/balances", "wallet/treasury", "wallet/payments"].map(p => `agents/${agentId}/${p}`)]) {
    const response = await request.get(`${protectedOrigin}/api/console/${path}`, { headers: { authorization } });
    expect(response.status(), path).toBe(200);
  }
});
test("mutations and unapproved paths are blocked before the backend", async ({ request }) => {
  for (const method of ["POST", "PUT", "PATCH", "DELETE"]) {
    const response = await request.fetch(`${protectedOrigin}/api/console/agents/${agentId}/cycles`, { method, headers: { authorization } });
    expect(response.status()).toBe(405);
  }
  for (const path of [`agents/${agentId}/wallet/sign`, `agents/${agentId}/wallet/recover`, `agents/${agentId}/wallet/policy`, "agents/not-a-uuid", "unknown", `agents/${agentId}/pause`]) {
    const response = await request.get(`${protectedOrigin}/api/console/${path}`, { headers: { authorization } });
    expect(response.status()).toBe(403);
  }
  expect((await request.get(`${protectedOrigin}/api/console/agents?target=elsewhere`, { headers: { authorization } })).status()).toBe(400);
});
test("cross-origin and DNS rebinding requests fail closed", async ({ request }) => {
  for (const headers of ([{ authorization, origin: "https://untrusted.invalid" }, { authorization, host: "untrusted.invalid" }] as Record<string, string>[])) {
    expect((await request.get(`${protectedOrigin}/api/console/agents`, { headers })).status()).toBe(403);
  }
});
test("disconnected proxy never forwards arbitrary targets", async ({ request }) => {
  const response = await request.get("/api/console/health"); expect(response.status()).toBe(503);
  expect(await response.json()).toEqual({ error: "backend_not_configured" });
});
