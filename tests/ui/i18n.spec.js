// Multi-language support (ADR-0014). English keeps the canonical
// unprefixed URLs; other locales live under /<code>/. These pin the
// parts that are easy to break silently: locale routing, that a
// translated page is actually translated, and the SEO structure that
// makes the whole exercise worthwhile — hreflang, canonical, html lang.
import { test, expect } from "@playwright/test";

test.describe("internationalisation", () => {
  test("English routes are unprefixed and unchanged", async ({ page }) => {
    await page.goto("/tool/merge-pdf/");
    await page.waitForSelector(".tool-head", { timeout: 45_000 });
    await expect(page.locator(".tool-head h1")).toHaveText("Merge PDF");
    expect(await page.evaluate(() => document.documentElement.lang)).toBe("en");
    await expect(page.locator(".lang-opt.active")).toHaveText("EN");
  });

  test("the Indonesian route renders Indonesian, not English", async ({ page }) => {
    await page.goto("/id/tool/merge-pdf/");
    await page.waitForSelector(".tool-head", { timeout: 45_000 });
    await expect(page.locator(".tool-head h1")).toHaveText("Gabung PDF");
    expect(await page.evaluate(() => document.documentElement.lang)).toBe("id");
    await expect(page.locator(".lang-opt.active")).toHaveText("ID");

    // Home and privacy too, so it isn't just tool names.
    await page.goto("/id/");
    await page.waitForSelector(".cat-chips", { timeout: 45_000 });
    await expect(page.locator("h1")).toContainText("Semua alat file");
    await expect(page.locator(".cat-chip").first()).toHaveText("Semua");
    await page.goto("/id/privacy/");
    await expect(page.locator("h1")).toContainText("Privasi");
  });

  test("a locale segment can never shadow a real route", async ({ page }) => {
    // If Locale::from_str accepted anything, /tool/... would parse as a
    // locale and every tool page would break.
    for (const path of ["/tool/compress-img/", "/privacy/", "/support/"]) {
      await page.goto(path);
      await page.waitForSelector(".nav", { timeout: 45_000 });
      expect(await page.evaluate(() => document.documentElement.lang)).toBe("en");
      await expect(page.locator(".lang-opt.active")).toHaveText("EN");
    }
  });

  test("the switcher moves between the same page in each language", async ({ page }) => {
    await page.goto("/tool/ocr-pdf/");
    await page.waitForSelector(".lang-switch", { timeout: 45_000 });
    await page.locator(".lang-opt", { hasText: "ID" }).click();
    await expect(page).toHaveURL(/\/id\/tool\/ocr-pdf/);
    await expect(page.locator(".tool-head h1")).toHaveText("OCR PDF");
    // …and back, to the unprefixed URL.
    await page.locator(".lang-opt", { hasText: "EN" }).click();
    await expect(page).toHaveURL(/\/tool\/ocr-pdf/);
    expect(new URL(page.url()).pathname.startsWith("/id/")).toBe(false);
  });

  // Canonical/hreflang/sitemap URLs use the BASE_URL baked in at build
  // time (the production origin), not the test server's address — so
  // discover it from the output rather than hardcoding a domain that
  // will change the day a real one is bought.
  async function bakedOrigin(page) {
    const html = await (await page.request.get("/")).text();
    const m = html.match(/rel="canonical" href="(https?:\/\/[^/"]+)/);
    expect(m, "home page has no canonical to derive the base URL from").toBeTruthy();
    return m[1];
  }

  test("prerendered pages carry the SEO structure that makes this worthwhile", async ({
    page,
  }) => {
    // Read the raw prerendered HTML, before the wasm app replaces it —
    // that is what a crawler sees.
    const res = await page.request.get("/id/tool/compress-pdf/");
    const html = await res.text();
    expect(html).toContain('<html lang="id"');
    expect(html).toMatch(/<title>[^<]*Kompres PDF[^<]*<\/title>/);

    const origin = await bakedOrigin(page);
    // Reciprocal hreflang, and each href must equal that page's canonical.
    expect(html).toContain(`hreflang="en" href="${origin}/tool/compress-pdf"`);
    expect(html).toContain(`hreflang="id" href="${origin}/id/tool/compress-pdf"`);
    expect(html).toContain(`hreflang="x-default" href="${origin}/tool/compress-pdf"`);
    expect(html).toContain(`rel="canonical" href="${origin}/id/tool/compress-pdf"`);

    // The English twin points back at the Indonesian one.
    const en = await (await page.request.get("/tool/compress-pdf/")).text();
    expect(en).toContain(`hreflang="id" href="${origin}/id/tool/compress-pdf"`);
    expect(en).toContain(`rel="canonical" href="${origin}/tool/compress-pdf"`);

    // A translated page must not leak English body copy.
    expect(html).toContain("Pertanyaan yang sering diajukan");
    expect(html).not.toContain("Frequently asked questions");
  });

  test("the sitemap lists every page in every language", async ({ page }) => {
    const xml = await (await page.request.get("/sitemap.xml")).text();
    const origin = await bakedOrigin(page);
    const locs = [...xml.matchAll(/<loc>([^<]+)<\/loc>/g)].map((m) => m[1]);
    // 38 tools + home + privacy + support, twice.
    expect(locs.length).toBe(82);
    expect(locs).toContain(`${origin}/tool/merge-pdf`);
    expect(locs).toContain(`${origin}/id/tool/merge-pdf`);
    expect(locs).toContain(`${origin}/id/`);
    // Every English tool URL has an Indonesian twin.
    const en = locs.filter((u) => u.includes("/tool/") && !u.includes("/id/"));
    for (const u of en) {
      expect(locs).toContain(u.replace(`${origin}/`, `${origin}/id/`));
    }
  });
});
