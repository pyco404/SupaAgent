import http from "node:http";
import { records } from "./fixtures.ts";

const server = http.createServer((request, response) => {
  if (request.url === "/ready") { response.end("ready"); return; }
  const authorized = request.headers.authorization === `Bearer ${"fixture-token-".repeat(4)}`;
  response.setHeader("Content-Type", "application/json");
  if (!authorized) { response.writeHead(401).end(JSON.stringify({ private_detail: "never-forward-upstream-errors" })); return; }
  if (request.method !== "GET") { response.writeHead(405).end("{}"); return; }
  const path = request.url?.slice(1) ?? "";
  if (!(path in records)) { response.writeHead(404).end("{}"); return; }
  // A canary proves unknown upstream fields are stripped before reaching the browser.
  const value = path === "health" ? { ...records[path], private_detail: "never-forward-unknown-fields" } : records[path];
  response.end(JSON.stringify(value));
});
server.listen(18081, "127.0.0.1");
