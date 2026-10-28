/** Server-only adapter. Reuses the operator console's Zod contracts and auth rules. */
import { readFile, stat } from 'node:fs/promises';
import { parseEnv } from 'node:util';
import { resolve } from 'node:path';
import { schemas } from '../web/src/lib/contracts.ts';
import { operatorAuthorized, localRequest } from '../web/src/lib/auth.ts';

export async function loadLocalEnvironment() {
  const path = process.env.SUPA_SITE_ENV_FILE ?? resolve(import.meta.dirname, '../web/.env.local');
  if (!path) return;
  try {
    const info = await stat(path);
    if (!info.isFile() || (info.mode & 0o077) !== 0 || info.uid !== process.getuid()) throw new Error('unsafe');
    const values = parseEnv(await readFile(path, 'utf8'));
    for (const key of ['SUPA_BACKEND_URL','SUPA_API_TOKEN','SUPA_WEB_USERNAME','SUPA_WEB_PASSWORD']) {
      if (process.env[key] === undefined && values[key] !== undefined) process.env[key] = values[key];
    }
  } catch (error) {
    if (error.code !== 'ENOENT') throw new Error('Local configuration must be an owner-only file');
  }
}
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
export function resource(path) {
  if (path === '/api/status') return { path:'/health', schema:schemas.health, public:true };
  if (path === '/api/agents') return { path:'/agents', schema:schemas.agents, public:true };
  const parts = path.split('/').slice(2);
  if (parts[0] !== 'agents' || !uuid.test(parts[1] ?? '')) return null;
  let schema;
  if (parts.length === 2) schema = schemas.agent;
  if (parts.length === 3 && ['financials','memory','goals','activity','cycles','actions','simulated-ledger','social','drafts','wallet'].includes(parts[2])) schema = schemas[parts[2]];
  if (parts.length === 4 && parts[2] === 'wallet' && ['balances','treasury','payments'].includes(parts[3])) schema = schemas[parts[3]];
  return schema ? { path:'/'+parts.join('/'), schema, public:false } : null;
}
const fail = (error, status) => ({ status, body:{ error } });
// Known configuration values and recognizable credentials never leave this adapter.
function scrub(value) {
  if (typeof value === 'string') {
    let text = value.replace(/(?:sk-[\w-]{12,}|Bearer\s+\S+|(?:postgres(?:ql)?|https?):\/\/[^\s/@]+:[^\s/@]+@\S+)/gi, '[redacted]');
    for (const [key, secret] of Object.entries(process.env)) {
      if (/(?:KEY|TOKEN|PASSWORD|SECRET|DATABASE_URL)$/.test(key) && secret?.length >= 8) text = text.split(secret).join('[redacted]');
    }
    return text;
  }
  if (Array.isArray(value)) return value.map(scrub);
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([k,v])=>[k,scrub(v)]));
  return value;
}
export async function readResource(path, headers, fetcher = fetch) {
  const request = new Request('http://127.0.0.1'+path, { headers });
  if (!localRequest(request)) return fail('local_access_only',403);
  if (path.includes('?')) return fail('query_not_permitted',400);
  const route = resource(path);
  if (!route) return fail('read_not_permitted',403);
  if (!route.public && !operatorAuthorized(request.headers.get('authorization'))) return fail('operator_authentication_required',401);
  const raw = process.env.SUPA_BACKEND_URL, token = process.env.SUPA_API_TOKEN;
  if (!raw || !token) return fail('backend_not_configured',503);
  let base;
  try {
    base = new URL(raw);
    if (base.protocol !== 'http:' || !['127.0.0.1','[::1]'].includes(base.hostname) || base.username || base.password || base.pathname !== '/' || base.search || base.hash || token.length < 32 || !/^[\x21-\x7e]+$/.test(token)) throw new Error();
  } catch { return fail('backend_configuration_invalid',503); }
  try {
    const response = await fetcher(new URL(route.path,base), { method:'GET', headers:{ Authorization:`Bearer ${token}`, Accept:'application/json' }, redirect:'error', cache:'no-store', signal:AbortSignal.timeout(5000) });
    if (!response.ok) return fail(({401:'backend_authentication_failed',403:'backend_permission_denied',404:'resource_not_found'})[response.status] ?? 'backend_unavailable', [401,403,404].includes(response.status)?response.status:502);
    if (!response.headers.get('content-type')?.includes('application/json')) return fail('backend_contract_invalid',502);
    const reader = response.body?.getReader();
    if (!reader) return fail('backend_contract_invalid',502);
    let size=0; const chunks=[];
    try { while (true) { const {done,value}=await reader.read(); if(done)break;size+=value.byteLength;if(size>2_000_000){await reader.cancel();return fail('backend_response_too_large',502);}chunks.push(value); } } finally { reader.releaseLock(); }
    const parsed = route.schema.safeParse(JSON.parse(Buffer.concat(chunks).toString('utf8')));
    if (!parsed.success) return fail('backend_contract_invalid',502);
    // Only this explicit public projection is declassified. No instructions, goals or memory.
    const body = path === '/api/agents' ? parsed.data.map(({id,name,lifecycle,created_at})=>({id,name,lifecycle,created_at})) : parsed.data;
    return { status:200, body:scrub(body) };
  } catch { return fail('backend_disconnected',503); }
}
export { operatorAuthorized, localRequest };
