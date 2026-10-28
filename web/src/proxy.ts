import { NextRequest, NextResponse } from "next/server";
import { localRequest, operatorAuthorized, stagingAuthorized } from "./lib/auth";

export function proxy(request: NextRequest) {
  if (process.env.SUPA_SITE_MODE === "staging" && request.nextUrl.pathname === "/healthz") {
    const headers = new Headers(request.headers); headers.set("x-supa-experience", "1");
    return NextResponse.next({ request: { headers } });
  }
  if (!localRequest(request)) return new NextResponse("Local access only", { status: 403 });
  if (process.env.SUPA_SITE_MODE === "staging") {
    const headers = new Headers(request.headers);
    headers.set("x-supa-experience", "1");
    if (request.nextUrl.pathname === "/healthz") return NextResponse.next({ request: { headers } });
    const authorization = request.headers.get("authorization");
    if (!operatorAuthorized(authorization) && !stagingAuthorized(authorization)) return new NextResponse("Private staging authentication required", {
      status: 401, headers: { "WWW-Authenticate": 'Basic realm="Supa staging", charset="UTF-8"', "Cache-Control": "no-store" },
    });
    const response = NextResponse.next({ request: { headers } });
    response.headers.set("Cache-Control", "private, no-store");
    return response;
  }
  // The public disconnected shell contains no operator data. Configuration activates authentication.
  if (!process.env.SUPA_BACKEND_URL && !process.env.SUPA_API_TOKEN) return NextResponse.next();
  if (!process.env.SUPA_WEB_USERNAME || (process.env.SUPA_WEB_PASSWORD?.length ?? 0) < 24) {
    return new NextResponse("Operator authentication is not configured", { status: 503 });
  }
  if (!operatorAuthorized(request.headers.get("authorization"))) {
    return new NextResponse("Operator authentication required", {
      status: 401, headers: { "WWW-Authenticate": 'Basic realm="Supa local operator", charset="UTF-8"', "Cache-Control": "no-store" },
    });
  }
  const response = NextResponse.next();
  response.headers.set("Cache-Control", "no-store");
  return response;
}
export const config = { matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"] };
