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
