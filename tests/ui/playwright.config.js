// Playwright config: serves the built web bundle (scripts/build-web.sh
// must have run) and tests the real wasm app in headless Chromium.
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  timeout: 60_000,
  retries: process.env.CI ? 1 : 0,
  workers: 1, // the wasm app is heavy; serialize for stability
  reporter: [["list"]],
  use: {
    baseURL: "http://127.0.0.1:4173",
    viewport: { width: 1440, height: 900 },
  },
  webServer: {
    command: "python3 ../../scripts/ui-server.py 4173 ../../target/dx/privzapp/release/web/public",
    url: "http://127.0.0.1:4173/",
    reuseExistingServer: true,
    timeout: 15_000,
  },
});
