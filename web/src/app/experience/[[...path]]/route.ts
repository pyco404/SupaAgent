/** Next.js hosts the approved experience unchanged; secrets stay in the server adapter. */
import { readFile, realpath } from "node:fs/promises";
import { extname, resolve, sep } from "node:path";
import { readResource, runAutonomy } from "../../../../../reference-web/backend.mjs";
import { operatorAuthorized } from "@/lib/auth";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";
export async function POST(request: Request, context: { params: Promise<{ path?: string[] }> }) {
  if(process.env.SUPA_SITE_MODE!=="staging")return new Response("Not found",{status:404});
  const path='/'+((await context.params).path??[]).join('/');
  if((path!=="/api/autonomy"&&!/^\/api\/agents\/[0-9a-f-]{36}\/(?:schedule|live(?:\/[0-9a-f-]{36}\/approval)?)$/i.test(path))||new URL(request.url).search)return new Response("Method not allowed",{status:405});
  if(!operatorAuthorized(request.headers.get("authorization")))return Response.json({error:"operator_authentication_required"},{status:401});
  const reader=request.body?.getReader();let size=0;const chunks:Uint8Array[]=[];
  if(!reader)return new Response("Invalid request",{status:400});
  try{while(true){const {done,value}=await reader.read();if(done)break;size+=value.byteLength;if(size>4096){await reader.cancel();return new Response("Request too large",{status:413});}chunks.push(value);}}finally{reader.releaseLock();}
  const result=await runAutonomy(path,request.headers,Buffer.concat(chunks).toString('utf8'));
  return Response.json(result.body,{status:result.status,headers:{"Cache-Control":"private, no-store"}});
}
const files = new Set(["index.html", "style.css", "app.js", "scene.js", "wheel.js", "platform.js"]);
const mime: Record<string, string> = { ".html":"text/html; charset=utf-8", ".css":"text/css", ".js":"text/javascript", ".svg":"image/svg+xml", ".woff2":"font/woff2", ".ttf":"font/ttf", ".txt":"text/plain" };
const headers = { "Cache-Control":"private, no-store", "X-Content-Type-Options":"nosniff" };
export async function GET(request: Request, context: { params: Promise<{ path?: string[] }> }) {
  if (process.env.SUPA_SITE_MODE !== "staging") return new Response("Not found", {status:404, headers});
  const parts = (await context.params).path ?? [];
  if (parts.some(part=>part.includes("\\") || part.includes("\0") || part === "." || part === "..")) return new Response("Invalid path", {status:400, headers});
  const path = "/"+parts.join("/");
  if (path === "/healthz") return Response.json({status:"ok",mode:"private-staging",signing_enabled:false},{headers});
  if (path === "/operator") {
    if (!operatorAuthorized(request.headers.get("authorization"))) return new Response("Operator authentication required", {status:401,headers:{...headers,"WWW-Authenticate":'Basic realm="Supa operator", charset="UTF-8"'}});
    return new Response(null, {status:302, headers:{...headers, Location:"/site/#agents"}});
  }
  if (path.startsWith("/api/")) {
    const result = await readResource(path+new URL(request.url).search, request.headers);
    return Response.json(result.body, {status:result.status,headers});
  }
  const file = ["/", "/site", "/site/"].includes(path) ? "index.html" : parts.join("/");
  const asset = /^public\/(assets\/[a-z0-9-]+\.svg|fonts\/[a-zA-Z0-9._-]+\.(woff2|ttf|txt))$/.test(file);
  if (!files.has(file) && !asset) return new Response("Not found", {status:404, headers});
  try {
    const root = await realpath(resolve(process.cwd(), "../reference-web"));
    const target = await realpath(resolve(root,file));
    if (!target.startsWith(root+sep)) return new Response("Not found", {status:404,headers});
    return new Response(await readFile(target), {headers:{...headers,"Content-Type":mime[extname(file)] ?? "application/octet-stream"}});
  } catch {return new Response("Asset unavailable", {status:503,headers});}
}
