// Anonymous page counting (ADR-0012). The beacon is a same-origin GET
// to /gc/count with the page path as its ONLY parameter — these tests
// pin the payload shape, the dedupe, the off toggle, and that the
// Global Privacy Control / Do-Not-Track signals silence it entirely.
// (The test server has no sidecar; the request itself is what matters.)
import { test, expect } from "@playwright/test";

function collectBeacons(page) {
  const hits = [];
  page.on("request", (r) => {
    const u = new URL(r.url());
    if (u.pathname === "/gc/count") hits.push(u);
  });
  return hits;
}

test.describe("visit counting", () => {
  test("sends exactly one path-only beacon per page view", async ({ page }) => {
    const hits = collectBeacons(page);
    await page.goto("/");
    await page.waitForSelector(".cat-chips", { timeout: 45_000 });
    await expect.poll(() => hits.length, { timeout: 10_000 }).toBe(1);
    expect(hits[0].searchParams.get("p")).toBe("/");
    // The path is the ONLY thing in the payload — no screen size, no
    // title, no referrer, no IDs. This list is the privacy contract.
    expect([...hits[0].searchParams.keys()]).toEqual(["p"]);

    // Navigating fires one more, with the new path.
    await page.locator(".tool-card", { has: page.locator("h3", { hasText: "Merge PDF" }) }).click();
    await expect.poll(() => hits.length, { timeout: 10_000 }).toBe(2);
    expect(hits[1].searchParams.get("p")).toBe("/tool/merge-pdf");
    for (const hit of hits) expect([...hit.searchParams.keys()]).toEqual(["p"]);

    // Re-renders on the same page (opening the mega menu) do not.
    await page.locator(".nav-alltools").click();
    await page.waitForTimeout(500);
    expect(hits.length).toBe(2);
  });

  test("the privacy-page toggle turns counting off, persistently", async ({ page }) => {
    await page.goto("/privacy");
    await page.waitForSelector("#analytics", { timeout: 45_000 });
    const toggle = page.getByLabel("Allow anonymous visit counting");
    await expect(toggle).toBeChecked();
    await toggle.uncheck();
    await expect
      .poll(() => page.evaluate(() => localStorage.getItem("pz-analytics")))
      .toBe("off");

    // Fresh navigation with counting off: zero beacons.
    const hits = collectBeacons(page);
    await page.goto("/");
    await page.waitForSelector(".cat-chips", { timeout: 45_000 });
    await page.waitForTimeout(1_000);
    expect(hits.length).toBe(0);

    // And the toggle remembers.
    await page.goto("/privacy");
    await expect(page.getByLabel("Allow anonymous visit counting")).not.toBeChecked();
  });

  test("Do-Not-Track silences the beacon too", async ({ browser }) => {
    const context = await browser.newContext();
    await context.addInitScript(() => {
      Object.defineProperty(navigator, "doNotTrack", { get: () => "1" });
    });
    const page = await context.newPage();
    const hits = collectBeacons(page);
    await page.goto("/");
    await page.waitForSelector(".cat-chips", { timeout: 45_000 });
    await page.waitForTimeout(1_000);
    expect(hits.length).toBe(0);
    await context.close();
  });

  test("Global Privacy Control silences the beacon with no toggle needed", async ({ browser }) => {
    const context = await browser.newContext();
    await context.addInitScript(() => {
      Object.defineProperty(navigator, "globalPrivacyControl", { get: () => true });
    });
    const page = await context.newPage();
    const hits = collectBeacons(page);
    await page.goto("/");
    await page.waitForSelector(".cat-chips", { timeout: 45_000 });
    await page.waitForTimeout(1_000);
    expect(hits.length).toBe(0);
    // The privacy page says so out loud (the note under the toggle).
    await page.goto("/privacy");
    await expect(page.locator(".opt .muted")).toContainText("counting is already off");
    await context.close();
  });
});
