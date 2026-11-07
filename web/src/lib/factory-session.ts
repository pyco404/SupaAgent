import { createHmac, timingSafeEqual } from "node:crypto";
import { operatorToken } from "./auth";

export const factoryEnabled = () => process.env.SUPA_SYNTHETIC_FACTORY_ENABLED === "true";
export const factoryCookie = "supa_factory_session";
type Account = { username: string; password: string; api_token: string };
function accounts(): Account[] {
  try {
    const values: unknown = JSON.parse(process.env.SUPA_WEB_OPERATORS ?? "[]");
    if (!Array.isArray(values) || values.length > 100) return [];
    return values.filter((a): a is Account => a && typeof a.username === "string" && typeof a.password === "string" && typeof a.api_token === "string");
  } catch { return []; }
}
function basic(a: Account) { return `Basic ${Buffer.from(`${a.username}:${a.password}`).toString("base64")}`; }
function signature(a: Account, payload: string) {
  return createHmac("sha256", a.api_token).update(`${a.password}\0${payload}`).digest("base64url");
}
// Only existing, explicitly provisioned synthetic accounts. No registration or owner grants.
export function issueFactorySession(username: string, password: string, now = Date.now()): string | null {
  if (!factoryEnabled() || username.length > 80 || password.length > 512 || !Number.isFinite(now)) return null;
  const header = `Basic ${Buffer.from(`${username}:${password}`).toString("base64")}`;
  if (!operatorToken(header)) return null;
  const a = accounts().find(a => a.username === username);
  if (!a) return null;
  const payload = Buffer.from(JSON.stringify({ username, expires: Math.floor(now / 1000) + 900 })).toString("base64url");
  return `${payload}.${signature(a, payload)}`;
}
export function factoryAuthorization(request: Request, now = Date.now()): string | null {
  if (!factoryEnabled()) return null;
  const direct = request.headers.get("authorization");
  if (operatorToken(direct)) return direct;
  const cookies = (request.headers.get("cookie") ?? "").split(";").map(c => c.trim()).filter(c => c.startsWith(`${factoryCookie}=`));
  if (cookies.length !== 1 || cookies[0].length > 2048 || !Number.isFinite(now)) return null;
  const value = cookies[0].slice(factoryCookie.length + 1);
  if (!/^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]{43}$/.test(value)) return null;
  try {
    const [payload, mac] = value.split(".");
    const data = JSON.parse(Buffer.from(payload, "base64url").toString("utf8"));
    const second = Math.floor(now / 1000);
    if (!Number.isSafeInteger(data.expires) || data.expires <= second || data.expires > second + 900) return null;
    const a = accounts().find(a => a.username === data.username);
    if (!a || !operatorToken(basic(a)) || !timingSafeEqual(Buffer.from(mac), Buffer.from(signature(a, payload)))) return null;
    return basic(a);
  } catch { return null; }
}
export function factorySameOrigin(request: Request): boolean {
  const host = request.headers.get("host"), origin = request.headers.get("origin");
  if (!host || !origin) return false;
  try {
    const source = new URL(origin);
    const allowed = [process.env.SUPA_SITE_ORIGIN, process.env.SUPA_PUBLIC_LANDING_ENABLED === "true" ? "https://supaagency.xyz" : undefined];
    return source.origin === origin && source.host === host && allowed.includes(source.origin);
  } catch { return false; }
}
