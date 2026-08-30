// "Zero telemetry" is a product claim on the home page, in the meta
// description and on the Privacy page — so it gets enforced, not just
// asserted. This drives the real bundle across every kind of page and
// fails if the app talks to ANY other host, or hits a same-origin path
// that looks like a counter. Analytics were built once and removed
// (ADR-0012); this is what stops them creeping back unnoticed.
import { test, expect } from "@playwright/test";
import { samplePng } from "./fixtures/sample-png.mjs";

// Anything that smells like a beacon, even from our own origin.
const COUNTER_PATH = /count|collect|analytic|telemetr|track|beacon|pixel|stat(s|us)?\b/i;

function watch(page, offHost, counters) {
  page.on("request", (r) => {
    const u = new URL(r.url());
    // blob:/data: are in-page, never network.
    if (u.protocol !== "http:" && u.protocol !== "https:") return;
    if (u.hostname !== "127.0.0.1" && u.hostname !== "localhost") {
      offHost.push(r.url());
      return;
    }
    if (COUNTER_PATH.test(u.pathname + u.search)) counters.push(r.url());
  });
}

test.describe("zero telemetry", () => {
  test("the app never talks to another host, on any page", async ({ page }) => {
    const offHost = [];
    const counters = [];
    watch(page, offHost, counters);

    await page.goto("/");
    await page.waitForSelector(".cat-chips", { timeout: 45_000 });
    // A plain tool, the editor (PDF.js), and the privacy page.
    await page.goto("/tool/compress-img/");
    await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
    await page.setInputFiles("#file-in", {
      name: "shot.png",
      mimeType: "image/png",
      buffer: samplePng(),
    });
    await page.waitForSelector(".preview-after", { timeout: 20_000 });
    await page.goto("/tool/edit-pdf/");
    await page.waitForSelector("#pdf-in", { state: "attached", timeout: 45_000 });
    await page.goto("/privacy");
    await page.waitForSelector(".prose", { timeout: 45_000 });
    await page.waitForTimeout(1_000);

    expect(offHost, `app contacted other hosts: ${offHost.join(", ")}`).toEqual([]);
    expect(counters, `app hit counter-ish paths: ${counters.join(", ")}`).toEqual([]);
  });

  test("the claim is on the page, and the privacy page backs it up", async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector(".cat-chips", { timeout: 45_000 });
    await expect(page.locator(".badge", { hasText: "Zero telemetry" })).toBeVisible();
    // Two description tags exist: the prerendered one seo-gen writes (what
    // crawlers read) and the one Dioxus sets at runtime. Both must carry
    // the claim.
    const descs = await page
      .locator('meta[name="description"]')
      .evaluateAll((els) => els.map((e) => e.content));
    expect(descs.length).toBeGreaterThanOrEqual(2);
    for (const d of descs) expect(d).toMatch(/zero telemetry/i);

    await page.goto("/privacy");
    await expect(page.locator("h2", { hasText: "What we collect: nothing" })).toBeVisible();
    await expect(page.locator(".prose")).toContainText("does not phone home");
  });
});
