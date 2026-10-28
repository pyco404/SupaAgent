import { defineConfig } from "../web/node_modules/@playwright/test/index.mjs";

export default defineConfig({
  testMatch: "**/*.spec.mjs",
  testDir: "./tests",
  workers: 1,
  fullyParallel: false,
  forbidOnly: true,
  retries: 0,
  reporter: [["list"], ["json", { outputFile: "test-results/results.json" }]],
  use: {
    baseURL: "http://127.0.0.1:3201",
    launchOptions: { executablePath: "/usr/bin/google-chrome" },
    colorScheme: "dark",
    reducedMotion: "reduce",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  projects: [
    { name: "desktop", use: { viewport: { width: 1440, height: 1000 } } },
    { name: "tablet", use: { viewport: { width: 768, height: 1024 } } },
    { name: "mobile", use: { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true } },
  ],
  webServer: [{ command: "node server.mjs", url: "http://127.0.0.1:3201/healthz", reuseExistingServer: false, env: { PORT: "3201", SUPA_SITE_ENV_FILE: "", SUPA_BACKEND_URL: "", SUPA_API_TOKEN: "", SUPA_WEB_USERNAME: "", SUPA_WEB_PASSWORD: "" } }, { command:"node tests/fixture-server.mjs", url:"http://127.0.0.1:3202/healthz", reuseExistingServer:false }],
});
