import { NextRequest, NextResponse } from "next/server";
import { localRequest, operatorAuthorized } from "./lib/auth";

export function proxy(request: NextRequest) {
  if (!localRequest(request)) return new NextResponse("Local access only", { status: 403 });
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
