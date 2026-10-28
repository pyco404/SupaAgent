import "server-only";
import { operatorToken } from "./auth";
import { schemas } from "./contracts";
import { z } from "zod";

const noCache = { "Cache-Control": "no-store" };
export function resolveResource(parts: string[]): { path: string; schema: z.ZodType } | null {
  if (parts.length === 1 && (parts[0] === "health" || parts[0] === "agents")) {
    return { path: `/${parts[0]}`, schema: schemas[parts[0]] };
  }
  if (parts[0] !== "agents" || !z.uuid().safeParse(parts[1]).success) return null;
  if (parts.length === 2) return { path: `/${parts.join("/")}`, schema: schemas.agent };
  const permitted = ["financials", "memory", "goals", "activity", "cycles", "actions", "simulated-ledger", "social", "drafts"] as const;
  if (parts.length === 3 && permitted.includes(parts[2] as typeof permitted[number])) {
    return { path: `/${parts.join("/")}`, schema: schemas[parts[2] as typeof permitted[number]] };
  }
  if (parts[2] === "wallet") {
    if (parts.length === 3) return { path: `/${parts.join("/")}`, schema: schemas.wallet };
    const reads = ["balances", "treasury", "payments"] as const;
    if (parts.length === 4 && reads.includes(parts[3] as typeof reads[number])) {
      return { path: `/${parts.join("/")}`, schema: schemas[parts[3] as typeof reads[number]] };
    }
  }
  return null;
}

export async function readBackend(parts: string[], authorization:string|null=null): Promise<Response> {
  const resource = resolveResource(parts);
  const fail = (error: string, status: number) => Response.json({ error }, { status, headers: noCache });
  if (!resource) return fail("read_not_permitted", 403);
  const raw = process.env.SUPA_BACKEND_URL, token = operatorToken(authorization);
  if (!raw || !token) return fail("backend_not_configured", 503);
  let base: URL;
  try {
    base = new URL(raw);
    if (base.protocol !== "http:" || !["127.0.0.1", "[::1]"].includes(base.hostname) || base.username || base.password || base.pathname !== "/" || base.search || base.hash || token.length < 32 || !/^[\x21-\x7e]+$/.test(token)) throw new Error();
  } catch { return fail("backend_configuration_invalid", 503); }
  try {
    const response = await fetch(new URL(resource.path, base), {
      method: "GET", headers: { Authorization: `Bearer ${token}`, Accept: "application/json" },
      cache: "no-store", redirect: "error", signal: AbortSignal.timeout(5000),
    });
    if (!response.ok) {
      const errors: Record<number, string> = { 401: "backend_authentication_failed", 403: "backend_permission_denied", 404: "resource_not_found" };
      return fail(errors[response.status] ?? "backend_unavailable", response.status in errors ? response.status : 502);
    }
    if (!response.headers.get("content-type")?.includes("application/json")) return fail("backend_contract_invalid", 502);
    // Read a bounded body; never forward arbitrary provider/debug/credential fields.
    const reader = response.body?.getReader();
    if (!reader) return fail("backend_contract_invalid", 502);
    const chunks: Uint8Array[] = []; let size = 0;
    try {
      while (true) {
        const { done, value } = await reader.read(); if (done) break;
        size += value.byteLength;
        if (size > 2_000_000) { await reader.cancel(); return fail("backend_response_too_large", 502); }
        chunks.push(value);
      }
    } finally { reader.releaseLock(); }
    const parsed = resource.schema.safeParse(JSON.parse(Buffer.concat(chunks).toString("utf8")));
    return parsed.success ? Response.json(parsed.data, { headers: noCache }) : fail("backend_contract_invalid", 502);
  } catch { return fail("backend_disconnected", 503); }
}
