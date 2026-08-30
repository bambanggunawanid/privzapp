// UI tests for the home page and navigation chrome.
import { test, expect } from "@playwright/test";

test.describe("home + nav", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector(".cat-chips", { timeout: 45_000 });
  });

  test("category filter chips narrow the grid", async ({ page }) => {
    await expect(page.locator(".tool-section")).toHaveCount(5);
    await page.locator(".cat-chip", { hasText: "Image" }).click();
    await expect(page.locator(".tool-section")).toHaveCount(1);
    await expect(page.locator(".tool-section h2")).toHaveText(/Image/);
    await page.locator(".cat-chip", { hasText: "All" }).click();
    await expect(page.locator(".tool-section")).toHaveCount(5);
  });

  test("every tool card shows its SVG tile icon", async ({ page }) => {
    // All 32 tools have a custom SVG (self-tiled, so no category-tint
    // class), and the srcs survive dx asset hashing. The emoji tile is
    // the fallback for a tool added without an icon — none left today.
    const cards = await page.locator(".tool-card").count();
    expect(cards).toBe(36);
    await expect(page.locator("img.tool-tile-svg")).toHaveCount(36);
    await expect(page.locator("span.tool-tile")).toHaveCount(0);
    for (const [name, slug] of [
      ["Merge PDF", "merge-pdf"],
      ["Strip Metadata", "strip-exif"],
      ["Extract ZIP", "unzip"],
      ["Encrypt File", "encrypt-file"],
    ]) {
      const tile = page
        .locator(".tool-card", { has: page.locator("h3", { hasText: name }) })
        .locator("img.tool-tile-svg");
      await expect(tile).toHaveAttribute("src", new RegExp(`${slug}.*\\.svg`));
    }
  });

  test("nav shows the star-on-GitHub button linking the repo", async ({ page }) => {
    const star = page.locator(".nav-links .gh-star");
    await expect(star).toBeVisible();
    await expect(star).toHaveAttribute(
      "href",
      "https://github.com/bambanggunawanid/privzapp",
    );
    // Outbound links must not leak an opener or referrer.
    await expect(star).toHaveAttribute("rel", /noopener/);
    await expect(star).toHaveAttribute("rel", /noreferrer/);
    await expect(star).toHaveAttribute("target", "_blank");
  });

  // The tile colour is baked into each SVG (they load via <img>, so page
  // CSS can't tint them) — this pins every icon to its category's tile so
  // a new one can't quietly ship in the wrong group's colour.
  test("tile icons are coloured by category group", async ({ page }) => {
    const TILE = {
      PDF: "#52304D",
      Image: "#24455C",
      Compress: "#4C3919", // ToolCategory::Archive
      Protect: "#40325E", // ToolCategory::Security
      Video: "#1C4B42",
    };
    const sections = await page.locator(".tool-section").count();
    expect(sections).toBe(5);
    let checked = 0;
    for (let i = 0; i < sections; i++) {
      const section = page.locator(".tool-section").nth(i);
      const heading = (await section.locator("h2").textContent()).trim();
      const group = Object.keys(TILE).find((k) => heading.startsWith(k));
      expect(group, `unmapped section heading: ${heading}`).toBeTruthy();
      const srcs = await section.locator("img.tool-tile-svg").evaluateAll((els) =>
        els.map((e) => e.getAttribute("src")),
      );
      expect(srcs.length).toBeGreaterThan(0);
      // Cards without art use the emoji tile and have no SVG to check.
      for (const src of srcs) {
        const svg = await (await page.request.get(src)).text();
        expect(svg, `${src} should carry the ${group} tile`).toContain(TILE[group]);
        checked++;
      }
    }
    expect(checked).toBe(36);
  });

  // Regression: on a phone the "All tools" and "Support us" labels wrapped
  // to a second line and spilled out of the fixed-height nav bar.
  test("phone nav: chips go icon-only and stay on one row", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(page.locator(".nav-alltools-label")).toBeHidden();
    await expect(page.locator(".support-label")).toBeHidden();
    await expect(page.locator(".gh-star-label")).toBeHidden();
    await expect(page.locator(".nav-alltools-glyph")).toBeVisible();
    await expect(page.locator(".support-glyph")).toBeVisible();

    const box = await page.evaluate(() => {
      const links = document.querySelector(".nav-links");
      const chips = [...links.children].map((e) => e.getBoundingClientRect());
      return {
        navHeight: Math.round(document.querySelector(".nav").getBoundingClientRect().height),
        linksOverflow: links.scrollWidth - links.clientWidth,
        rightEdge: Math.round(Math.max(...chips.map((r) => r.right))),
        clientWidth: document.documentElement.clientWidth,
        docOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      };
    });
    expect(box.navHeight).toBe(58);
    expect(box.linksOverflow).toBe(0);
    expect(box.docOverflow).toBe(0);
    expect(box.rightEdge).toBeLessThanOrEqual(box.clientWidth);

    // Icon-only, but still named for screen readers — and still opens.
    await expect(page.locator(".nav-alltools")).toHaveAttribute("aria-label", "All tools");
    await expect(page.locator(".support-cta")).toHaveAttribute("aria-label", "Support us");
    await page.locator(".nav-alltools").click();
    await expect(page.locator(".megamenu")).toBeVisible();
  });

  test("all-tools mega menu opens and navigates", async ({ page }) => {
    await page.locator(".nav-alltools").click();
    await expect(page.locator(".megamenu")).toBeVisible();
    await page.locator(".megamenu a", { hasText: "Merge PDF" }).click();
    await expect(page).toHaveURL(/\/tool\/merge-pdf/);
    await expect(page.locator(".megamenu")).toHaveCount(0);
  });
});
