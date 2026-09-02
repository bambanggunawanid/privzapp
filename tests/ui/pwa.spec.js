// PWA: service-worker registration and offline capability.
//
// This was a manual checklist item, on the assumption that install /
// activate / offline cycles can't be driven headlessly. They can: the
// registration is observable from the page, and CDP's
// Network.emulateNetworkConditions pulls the plug for real.
//
// Worth automating because the failure was silent. The app registered
// the worker from a `load` listener added *after* the wasm booted — by
// which time `load` had already fired — so the worker was never
// registered at all and "works offline" was simply untrue, with nothing
// on screen to say so.
import { test, expect } from "@playwright/test";

test.setTimeout(120_000);

async function goOffline(page, offline) {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Network.enable");
  await cdp.send("Network.emulateNetworkConditions", {
    offline,
    latency: 0,
    downloadThroughput: -1,
    uploadThroughput: -1,
  });
  return cdp;
}

test.describe("PWA", () => {
  test("the service worker actually registers and activates", async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector(".cat-chips", { timeout: 45_000 });
    await page.evaluate(() => navigator.serviceWorker.ready);

    const reg = await page.evaluate(async () => {
      const rs = await navigator.serviceWorker.getRegistrations();
      return { count: rs.length, state: rs[0]?.active?.state, scope: rs[0]?.scope };
    });
    expect(reg.count, "no service worker registered — offline support is dead").toBe(1);
    expect(reg.state).toBe("activated");
    // Scope must be the origin root, or it can't control the whole app.
    expect(new URL(reg.scope).pathname).toBe("/");
  });

  test("the app still loads with the network disconnected", async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector(".cat-chips", { timeout: 45_000 });
    await page.evaluate(() => navigator.serviceWorker.ready);
    // Visit a tool page so its shell is in the runtime cache too.
    await page.goto("/tool/merge-pdf/");
    await page.waitForSelector(".tool-head", { timeout: 45_000 });
    await page.waitForTimeout(2_000);

    const cdp = await goOffline(page, true);
    try {
      await page.reload({ waitUntil: "domcontentloaded", timeout: 30_000 });
      // The wasm app boots from cache, not just a cached HTML shell.
      await expect(page.locator(".tool-head, .cat-chips").first()).toBeVisible({
        timeout: 30_000,
      });
      // And navigation within the app keeps working offline.
      await page.goto("/");
      await expect(page.locator(".cat-chips")).toBeVisible({ timeout: 30_000 });
    } finally {
      await cdp.send("Network.emulateNetworkConditions", {
        offline: false,
        latency: 0,
        downloadThroughput: -1,
        uploadThroughput: -1,
      });
    }
  });

  test("the manifest and icons are served from the origin root", async ({ page }) => {
    // A service worker's scope is capped at its own path, which is why
    // these are copied to the root rather than hashed into /assets/.
    for (const path of ["/sw.js", "/manifest.webmanifest", "/apple-touch-icon.png"]) {
      const res = await page.request.get(path);
      expect(res.status(), `${path} must be served from the root`).toBe(200);
    }
    const manifest = await (await page.request.get("/manifest.webmanifest")).json();
    expect(manifest.name || manifest.short_name).toContain("PrivZapp");
    expect(manifest.icons.length).toBeGreaterThan(0);
  });
});
