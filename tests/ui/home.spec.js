// UI tests for the home page and navigation chrome.
import { test, expect } from "@playwright/test";

test.describe("home + nav", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector(".cat-chips", { timeout: 45_000 });
  });

  test("category filter chips narrow the grid", async ({ page }) => {
    await expect(page.locator(".tool-section")).toHaveCount(4);
    await page.locator(".cat-chip", { hasText: "Image" }).click();
    await expect(page.locator(".tool-section")).toHaveCount(1);
    await expect(page.locator(".tool-section h2")).toHaveText(/Image/);
    await page.locator(".cat-chip", { hasText: "All" }).click();
    await expect(page.locator(".tool-section")).toHaveCount(4);
  });

  test("PDF tools show their SVG tile icons; others keep the emoji tile", async ({ page }) => {
    // All 14 PDF-tool cards render the custom SVG (self-tiled, so no
    // category-tint class), and the src survives dx asset hashing.
    await expect(page.locator("img.tool-tile-svg")).toHaveCount(14);
    const merge = page
      .locator(".tool-card", { has: page.locator("h3", { hasText: "Merge PDF" }) })
      .locator("img.tool-tile-svg");
    await expect(merge).toHaveAttribute("src", /merge-pdf.*\.svg/);
    // Non-PDF categories still use the emoji tiles until their sets land.
    await expect(page.locator("span.tool-tile.cat-image").first()).toBeVisible();
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

  test("all-tools mega menu opens and navigates", async ({ page }) => {
    await page.locator(".nav-alltools").click();
    await expect(page.locator(".megamenu")).toBeVisible();
    await page.locator(".megamenu a", { hasText: "Merge PDF" }).click();
    await expect(page).toHaveURL(/\/tool\/merge-pdf/);
    await expect(page.locator(".megamenu")).toHaveCount(0);
  });
});
