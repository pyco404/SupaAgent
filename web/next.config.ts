import type { NextConfig } from "next";
import { resolve } from "node:path";

const config: NextConfig = {
  poweredByHeader: false,
  devIndicators: false,
  outputFileTracingRoot: resolve(process.cwd(), ".."),
  async rewrites() {
    return { beforeFiles: [{
      source: "/:path((?!experience(?:/|$)|_next(?:/|$)).*)",
      has: [{ type: "header", key: "x-supa-experience", value: "1" }],
      destination: "/experience/:path",
    }] };
  },
  async headers() {
    return [{ source: "/:path*", headers: [
      { key: "X-Content-Type-Options", value: "nosniff" },
      { key: "X-Frame-Options", value: "DENY" },
      { key: "Referrer-Policy", value: "no-referrer" },
      { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=()" },
      { key: "Content-Security-Policy", value: `default-src 'self'; script-src 'self' 'unsafe-inline'${process.env.NODE_ENV === "development" ? " 'unsafe-eval'" : ""}; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; object-src 'none'; base-uri 'self'; form-action 'self'` },
    ] }];
  },
};
export default config;
