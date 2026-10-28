import { createHash, timingSafeEqual } from "node:crypto";

export function operatorAuthorized(header: string | null): boolean {
  const username = process.env.SUPA_WEB_USERNAME;
  const password = process.env.SUPA_WEB_PASSWORD;
  if (!username || !password || password.length < 24) return false;
  if (!header?.startsWith("Basic ") || header.length > 2048) return false;
  const hash = (value: string) => createHash("sha256").update(value).digest();
  return timingSafeEqual(hash(header), hash(`Basic ${Buffer.from(`${username}:${password}`).toString("base64")}`));
}

export function localRequest(request: Request): boolean {
  if (process.env.SUPA_SITE_MODE === "staging") {
    try {
      const configured = new URL(process.env.SUPA_SITE_ORIGIN ?? "");
      const loopback = ["127.0.0.1", "localhost", "[::1]"].includes(configured.hostname);
      if ((configured.protocol !== "https:" && !(loopback && configured.protocol === "http:")) || configured.username || configured.password || configured.pathname !== "/" || configured.search || configured.hash) return false;
      const origin = request.headers.get("origin");
      return request.headers.get("host") === configured.host && (!origin || origin === configured.origin);
    } catch { return false; }
  }
  const allowed = (value: string) => {
    try {
      const url = new URL(value);
      return url.protocol === "http:" && ["127.0.0.1", "localhost", "[::1]"].includes(url.hostname) && !url.username && !url.password;
    } catch { return false; }
  };
  const host = request.headers.get("host");
  if (!host || !allowed(`http://${host}`)) return false;
  const origin = request.headers.get("origin");
  return !origin || (allowed(origin) && new URL(origin).host === host);
}

/** Separate staging visitors can browse public identities without reading operator records. */
export function stagingAuthorized(header: string | null): boolean {
  const username = process.env.SUPA_STAGING_USERNAME, password = process.env.SUPA_STAGING_PASSWORD;
  if (!username || !password || password.length < 24 || username === process.env.SUPA_WEB_USERNAME || !header?.startsWith("Basic ") || header.length > 2048) return false;
  const digest = (value: string) => createHash("sha256").update(value).digest();
  return timingSafeEqual(digest(header), digest(`Basic ${Buffer.from(`${username}:${password}`).toString("base64")}`));
}
