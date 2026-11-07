import { createHash, createHmac } from "node:crypto";
import { z } from "zod";
import { factoryAuthorization, factoryEnabled, factorySameOrigin } from "./factory-session";
import { localRequest, operatorToken } from "./auth";
import { AvatarImageError, defaultAvatar, normalizeAvatar, readAvatarBody, withAvatarSlot } from "./avatar-images";

export const avatarsEnabled = () => factoryEnabled() && process.env.SUPA_AGENT_AVATARS_ENABLED === "true";
const uuid = z.uuid(), digest = z.string().regex(/^[0-9a-f]{64}$/);
const upload = z.object({ asset_id: uuid, sha256: digest, kind: z.enum(["upload", "default"]) }).strict();
const metadata = z.object({ default_seed: uuid, revision: z.number().int().min(0).max(50), asset_id: uuid.nullable(), sha256: digest.nullable(), kind: z.enum(["upload", "default"]), token_image: z.object({ revision: z.number().int().positive(), sha256: digest }).strict().nullable() }).strict();
const profile = z.object({ agent_id: uuid, name: z.string().min(1).max(80), summary: z.string().min(1).max(280), revision: z.number().int().positive() }).strict();
const noCache = { "Cache-Control": "private, no-store", "X-Content-Type-Options": "nosniff", Vary: "Cookie, Authorization" };
const fail = (error: string, status: number) => Response.json({ error }, { status, headers: noCache });
class AvatarRequestError extends Error { constructor(public status: number) { super("avatar_request_denied_or_unavailable"); } }
export async function avatarBackend(request: Request, path: string, body?: unknown, publicRead = false): Promise<unknown> {
  const token = publicRead ? process.env.SUPA_API_TOKEN : operatorToken(factoryAuthorization(request));
  if (!token) throw new AvatarRequestError(401);
  const base = new URL(process.env.SUPA_BACKEND_URL ?? "");
  if (base.protocol !== "http:" || !(["127.0.0.1", "[::1]"].includes(base.hostname) || (process.env.SUPA_SITE_MODE === "staging" && /^[a-z0-9][a-z0-9-]*\.railway\.internal$/.test(base.hostname))) || base.username || base.password || base.pathname !== "/" || base.search || base.hash) throw new AvatarRequestError(503);
  const response = await fetch(new URL(path, base), { method: body === undefined ? "GET" : "POST", headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json", Accept: "application/json" }, body: body === undefined ? undefined : JSON.stringify(body), redirect: "error", cache: "no-store", signal: AbortSignal.timeout(10000) });
  if (!response.ok) { await response.body?.cancel(); throw new AvatarRequestError([400, 401, 403, 404, 409, 429].includes(response.status) ? response.status : 503); }
  if (!response.headers.get("content-type")?.includes("application/json")) throw new AvatarRequestError(502);
  const reader = response.body?.getReader(); if (!reader) throw new AvatarRequestError(502);
  let size = 0; const chunks: Uint8Array[] = [];
  try { while (true) { const part = await reader.read(); if (part.done) break; size += part.value.length; if (size > 2_300_000) { await reader.cancel(); throw new AvatarRequestError(502); } chunks.push(part.value); } }
  finally { reader.releaseLock(); }
  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
}
export async function stageAvatar(request: Request, bytes: Buffer, kind: "default" | "upload", requestId: string) {
  const token = operatorToken(factoryAuthorization(request)), service = process.env.SUPA_API_TOKEN;
  if (!token || !service || service.length < 32) throw new AvatarRequestError(503);
  const sha256 = createHash("sha256").update(bytes).digest("hex");
  const proof = createHmac("sha256", service).update(`agent-avatar-v1\n${token}\n${requestId}\n${sha256}\n${kind}`).digest("hex");
  return upload.parse(await avatarBackend(request, "/factory/avatars", { operation: "upload", request_id: requestId, kind, sha256, png_hex: bytes.toString("hex"), proof }));
}
const attempts = new Map<string, { since: number; count: number }>();
function admit(request: Request, category = "command") {
  const token = operatorToken(factoryAuthorization(request)); if (!token) throw new AvatarRequestError(401);
  const key = createHash("sha256").update(token + category).digest("hex"), now = Date.now();
  for (const [id, row] of attempts) if (now - row.since >= 60000) attempts.delete(id);
  const row = attempts.get(key) ?? { since: now, count: 0 };
  if (row.count >= (category === "upload" ? 10 : 30) || (!attempts.has(key) && attempts.size >= 100)) throw new AvatarRequestError(429);
  row.count++; attempts.set(key, row);
}
function imageResponse(data: unknown) {
  const value = z.object({ png_hex: z.string().regex(/^[0-9a-f]+$/).max(2_200_000), sha256: digest }).strict().parse(data);
  const bytes = Buffer.from(value.png_hex, "hex");
  if (createHash("sha256").update(bytes).digest("hex") !== value.sha256) throw new AvatarRequestError(502);
  return new Response(new Uint8Array(bytes), { headers: { ...noCache, "Content-Type": "image/png", "Content-Disposition": 'inline; filename="agent-avatar.png"', "Content-Security-Policy": "default-src 'none'; sandbox" } });
}
export async function avatarRoute(request: Request, path: string): Promise<Response> {
  if (!avatarsEnabled() || !localRequest(request) || new URL(request.url).search) return fail("avatars_unavailable", 403);
  const isPublic = path.startsWith("/api/public/");
  if (isPublic && request.method !== "GET") return fail("read_only", 405);
  if (!isPublic && !factoryAuthorization(request)) return fail("sign_in_required", 401);
  if (request.method !== "GET" && !factorySameOrigin(request)) return fail("same_origin_required", 403);
  try {
    const parts = path.split("/");
    if (isPublic) {
      if (path === "/api/public/agent-profiles") return Response.json(z.object({ agents: z.array(profile).max(100) }).strict().parse(await avatarBackend(request, "/public/agent-profiles", undefined, true)), { headers: noCache });
      const id = uuid.parse(parts[4]);
      if (parts[3] === "agent-profiles" && parts.length === 5) return Response.json(profile.parse(await avatarBackend(request, `/public/agent-profiles/${id}`, undefined, true)), { headers: noCache });
      if (parts[3] === "agent-avatars" && parts.length === 6 && /^[1-9][0-9]?$/.test(parts[5])) return imageResponse(await avatarBackend(request, `/public/agent-avatars/${id}/${parts[5]}`, undefined, true));
    } else if (parts[3] === "avatar-view" && parts.length === 5 && request.method === "GET") {
      const id=uuid.parse(parts[4]), meta=metadata.parse(await avatarBackend(request, `/factory/avatars/${id}`));
      if(meta.asset_id) return imageResponse(await avatarBackend(request, `/factory/avatar-assets/${meta.asset_id}`));
      const bytes=await withAvatarSlot(()=>defaultAvatar(meta.default_seed));
      return imageResponse({png_hex:bytes.toString("hex"),sha256:createHash("sha256").update(bytes).digest("hex")});
    } else if (parts[3] === "avatar-default" && parts.length === 5 && ["GET","POST"].includes(request.method)) {
      const id=uuid.parse(parts[4]);
      if(request.method === "POST") {
        admit(request,"upload");
        const requestId=uuid.parse(request.headers.get("x-request-id"));
        return Response.json(await withAvatarSlot(async()=>stageAvatar(request,await defaultAvatar(id),"default",requestId)),{headers:noCache});
      }
      const bytes = await withAvatarSlot(() => defaultAvatar(uuid.parse(parts[4])));
      return imageResponse({ png_hex: bytes.toString("hex"), sha256: createHash("sha256").update(bytes).digest("hex") });
    } else if (parts[3] === "avatar-uploads" && parts.length === 4 && request.method === "POST") {
      admit(request,"upload");
      // Verify current role/session before accepting expensive decoder work.
      await avatarBackend(request, "/factory/agents");
      const requestId = uuid.parse(request.headers.get("x-request-id"));
      const crop = request.headers.get("x-avatar-crop");
      const square = crop ? z.object({ left: z.number().int().nonnegative(), top: z.number().int().nonnegative(), size: z.number().int().positive().max(4096) }).strict().parse(JSON.parse(crop)) : undefined;
      const result = await withAvatarSlot(async () => stageAvatar(request, await normalizeAvatar(await readAvatarBody(request), request.headers.get("content-type") ?? "", square), "upload", requestId));
      return Response.json(result, { headers: noCache });
    } else if (parts[3] === "avatar-assets" && parts.length === 5) {
      const id = uuid.parse(parts[4]);
      if (request.method === "GET") return imageResponse(await avatarBackend(request, `/factory/avatar-assets/${id}`));
      if (request.method === "DELETE") return Response.json(z.object({ removed: z.boolean() }).strict().parse(await avatarBackend(request, "/factory/avatars", { operation: "remove_upload", asset_id: id })), { headers: noCache });
    } else if (parts[3] === "avatars" && parts.length === 5 && request.method === "GET") return Response.json(metadata.parse(await avatarBackend(request, `/factory/avatars/${uuid.parse(parts[4])}`)), { headers: noCache });
    else if (path === "/api/factory/avatars" && request.method === "POST") {
      admit(request);
      if (request.headers.get("content-type") !== "application/json") return fail("json_required", 415);
      const reader = request.body?.getReader(); if (!reader) return fail("body_required", 400);
      const chunks: Uint8Array[] = []; let size = 0;
      try { while (true) { const part = await reader.read(); if (part.done) break; size += part.value.length; if (size > 2048) { await reader.cancel(); return fail("body_too_large", 413); } chunks.push(part.value); } } finally { reader.releaseLock(); }
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8"));
      const common = { request_id: uuid, agent_id: uuid, expected_revision: z.number().int().min(0).max(50) };
      const command = z.discriminatedUnion("operation", [
        z.object({ ...common, operation: z.literal("set_avatar"), asset_id: uuid }).strict(),
        z.object({ ...common, operation: z.literal("approve_public"), approved: z.boolean() }).strict(),
        z.object({ ...common, operation: z.literal("confirm_token_image"), sha256: digest, confirmed: z.literal(true) }).strict(),
        z.object({ operation: z.literal("cleanup") }).strict(),
      ]).parse(body);
      const result = await avatarBackend(request, "/factory/avatars", command);
      const contract = command.operation === "set_avatar" ? z.object({ revision: z.number().int().positive(), sha256: digest, asset_id: uuid, kind: z.enum(["upload", "default"]) }).strict()
        : command.operation === "approve_public" ? z.object({ avatar_approved: z.boolean(), agent_listing_unchanged: z.literal(true) }).strict()
          : command.operation === "cleanup" ? z.object({ removed: z.number().int().nonnegative() }).strict() : z.object({ revision: z.number().int().positive(), sha256: digest, launch_enabled: z.literal(false) }).strict();
      return Response.json(contract.parse(result), { headers: noCache });
    }
    return fail("avatar_route_not_found", 404);
  } catch (error) {
    if (error instanceof AvatarImageError) return fail(error.code, error.status);
    if (error instanceof AvatarRequestError) return fail(error.message, error.status);
    return fail("invalid_or_unavailable_avatar", error instanceof z.ZodError || error instanceof SyntaxError ? 400 : 503);
  }
}
