import { createServer } from "node:http";
import { readFile, realpath, stat } from "node:fs/promises";
import { dirname, extname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const port = Number(process.env.PORT ?? 3200);
if (!Number.isInteger(port) || port < 1024 || port > 65535) throw new Error("Invalid local port");
const appFiles = new Set(["index.html", "style.css", "app.js", "scene.js", "wheel.js", "reference-data.json"]);
const media = new Map([
  [".html", "text/html; charset=utf-8"], [".css", "text/css; charset=utf-8"], [".txt", "text/plain; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"], [".json", "application/json; charset=utf-8"], [".woff2", "font/woff2"],
  [".woff", "font/woff"], [".ttf", "font/ttf"], [".png", "image/png"],
  [".jpg", "image/jpeg"], [".jpeg", "image/jpeg"], [".webp", "image/webp"],
  [".avif", "image/avif"], [".svg", "image/svg+xml"], [".ico", "image/x-icon"],
]);
const publicExtensions = new Set([".woff2", ".woff", ".ttf", ".png", ".jpg", ".jpeg", ".webp", ".avif", ".svg", ".ico"]);

function finish(response, status, message) {
  response.writeHead(status, { "Content-Type": "text/plain; charset=utf-8" });
  response.end(message);
}

createServer(async (request, response) => {
  response.setHeader("X-Content-Type-Options", "nosniff");
  response.setHeader("Referrer-Policy", "no-referrer");
  response.setHeader("Cache-Control", "no-store");
  response.setHeader("Content-Security-Policy", "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' https://hancore.dev; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'");
  if (request.method !== "GET" && request.method !== "HEAD") {
    response.setHeader("Allow", "GET, HEAD");
    return finish(response, 405, "Read-only preview");
  }
  let path;
  try { path = decodeURIComponent((request.url ?? "/").split("?", 1)[0]); }
  catch { return finish(response, 400, "Invalid path"); }
  if (path.includes("\0") || path.includes("\\") || path.split("/").some(part => part === "." || part === "..")) {
    return finish(response, 400, "Invalid path");
  }
  if (path === "/healthz") {
    response.writeHead(200, { "Content-Type": "application/json; charset=utf-8" });
    return response.end(request.method === "HEAD" ? undefined : JSON.stringify({ status: "ok", mode: "isolated-reference", host: "127.0.0.1" }));
  }
  const file = path === "/" || path === "/site" || path === "/site/" ? "index.html" : path.slice(1);
  const isPublic = file.startsWith("public/") && file.split("/").every(part => part && !part.startsWith(".")) && (publicExtensions.has(extname(file)) || file === "public/fonts/OFL.txt");
  if (!appFiles.has(file) && !isPublic) return finish(response, 404, "Not found");
  try {
    const target = await realpath(resolve(root, file));
    const safeRoot = isPublic ? resolve(root, "public") : root;
    if (!target.startsWith(`${safeRoot}${sep}`) || !(await stat(target)).isFile()) return finish(response, 404, "Not found");
    const content = await readFile(target);
    response.writeHead(200, { "Content-Type": media.get(extname(file)), "Content-Length": content.byteLength });
    response.end(request.method === "HEAD" ? undefined : content);
  } catch (error) {
    if (error.code === "ENOENT" || error.code === "ENOTDIR" || error.code === "EACCES") return finish(response, 404, "Not found");
    finish(response, 500, "Local asset unavailable");
  }
}).listen(port, "127.0.0.1", () => console.log(`Isolated reference preview: http://127.0.0.1:${port}`));
