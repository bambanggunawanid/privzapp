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

  test("all-tools mega menu opens and navigates", async ({ page }) => {
    await page.locator(".nav-alltools").click();
    await expect(page.locator(".megamenu")).toBeVisible();
    await page.locator(".megamenu a", { hasText: "Merge PDF" }).click();
    await expect(page).toHaveURL(/\/tool\/merge-pdf/);
    await expect(page.locator(".megamenu")).toHaveCount(0);
  });
});
