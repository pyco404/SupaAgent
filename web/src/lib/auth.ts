import { createHash, timingSafeEqual } from "node:crypto";

// Each Basic account maps to its own server-side Rust credential. No identity headers.
export function operatorToken(header:string|null):string|null {
  if (!header?.startsWith("Basic ") || header.length>2048) return null;
  const accounts: {username:string;password:string;api_token:string}[]=[];
  if(process.env.SUPA_WEB_USERNAME && process.env.SUPA_WEB_PASSWORD && process.env.SUPA_API_TOKEN)
    accounts.push({username:process.env.SUPA_WEB_USERNAME,password:process.env.SUPA_WEB_PASSWORD,api_token:process.env.SUPA_API_TOKEN});
  try {
    const configured:unknown=JSON.parse(process.env.SUPA_WEB_OPERATORS??"[]");
    if(!Array.isArray(configured)||configured.length>100) return null;
    for(const account of configured) {
      if(!account || typeof account.username!=="string" || typeof account.password!=="string" || typeof account.api_token!=="string") return null;
      accounts.push(account);
    }
  } catch { return null; }
  if(new Set(accounts.map(a=>a.username)).size!==accounts.length || new Set(accounts.map(a=>a.api_token)).size!==accounts.length) return null;
  const hash=(value:string)=>createHash("sha256").update(value).digest();
  for(const account of accounts) {
    if(account.password.length<24 || account.api_token.length<32 || account.api_token.length>256 || !/^[\x21-\x7e]+$/.test(account.api_token)) continue;
    if(timingSafeEqual(hash(header),hash(`Basic ${Buffer.from(`${account.username}:${account.password}`).toString("base64")}`))) return account.api_token;
  }
  return null;
}
export function operatorAuthorized(header: string | null): boolean { return operatorToken(header)!==null; }

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
