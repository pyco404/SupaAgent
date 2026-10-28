import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests", fullyParallel: true, workers: 3, forbidOnly: true, retries: 0,
  reporter: [["list"], ["json", { outputFile: "test-results/results.json" }]],
  use: { baseURL: "http://127.0.0.1:3100", launchOptions: { executablePath: "/usr/bin/google-chrome" }, trace: "retain-on-failure", screenshot: "only-on-failure", colorScheme: "dark", reducedMotion: "reduce" },
  projects: [
    { name: "desktop", use: { viewport: { width: 1440, height: 1000 } } },
    { name: "tablet", use: { viewport: { width: 768, height: 1024 } } },
    { name: "mobile", use: { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true } },
  ],
  webServer: [
    { command: "node tests/fixture-server.mjs", url: "http://127.0.0.1:18081/ready", reuseExistingServer: false },
    { command: "npm run start -- --port 3100", url: "http://127.0.0.1:3100", reuseExistingServer: false, env: { SUPA_BACKEND_URL: "", SUPA_API_TOKEN: "", SUPA_WEB_USERNAME: "", SUPA_WEB_PASSWORD: "" } },
    { command: "npm run start -- --port 3101", url: "http://127.0.0.1:3101", ignoreHTTPSErrors: false, reuseExistingServer: false, env: { SUPA_BACKEND_URL: "http://127.0.0.1:18081", SUPA_API_TOKEN: "fixture-token-".repeat(4), SUPA_WEB_USERNAME: "fixture", SUPA_WEB_PASSWORD: "local-test-only-".repeat(3) } },
  ],
});
