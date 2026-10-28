import { readBackend } from "@/lib/backend";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";
export async function GET(request: Request, context: { params: Promise<{ path: string[] }> }) {
  if (new URL(request.url).search) return Response.json({ error: "query_not_permitted" }, { status: 400 });
  return readBackend((await context.params).path,request.headers.get("authorization"));
}
