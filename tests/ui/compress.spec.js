// UI tests for the Compress Image tool. Each pins an owner-reported bug:
// missing per-file thumbnails, freezing while sliding quality, and the
// preview surviving Clear.
import { test, expect } from "@playwright/test";
import { samplePng } from "./fixtures/sample-png.mjs";

function pngFiles(n) {
  return Array.from({ length: n }, (_, i) => ({
    name: `photo-${i + 1}.png`,
    mimeType: "image/png",
    buffer: samplePng(),
  }));
}

test.describe("compress image", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tool/compress-img/");
    await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
  });

  test("multiple uploads show one thumbnail per image", async ({ page }) => {
    await page.setInputFiles("#file-in", pngFiles(3));
    await expect(page.locator(".thumb-card")).toHaveCount(3);
    const srcs = await page.locator(".thumb-card img").evaluateAll((imgs) =>
      imgs.map((i) => i.src),
    );
    expect(srcs).toHaveLength(3);
    for (const src of srcs) expect(src).toMatch(/^blob:/);
    await expect(page.locator(".preview")).toBeVisible({ timeout: 15_000 });
  });

  test("quality +/- buttons step by 10", async ({ page }) => {
    await page.setInputFiles("#file-in", pngFiles(1));
    await expect(page.locator(".opt label").first()).toHaveText("Quality: 80");
    await page.locator('button[title="Quality −10"]').click();
    await expect(page.locator(".opt label").first()).toHaveText("Quality: 70");
    await page.locator('button[title="Quality −10"]').click();
    await expect(page.locator(".opt label").first()).toHaveText("Quality: 60");
    await page.locator('button[title="Quality +10"]').click();
    await expect(page.locator(".opt label").first()).toHaveText("Quality: 70");
  });

  test("dragging the slider does not recompress until release (freeze regression)", async ({ page }) => {
    await page.setInputFiles("#file-in", pngFiles(1));
    await expect(page.locator(".preview p")).toContainText("→", { timeout: 15_000 });
    // The preview blob URL is recreated on every engine run — a
    // format-independent "did it recompute?" probe.
    const srcBefore = await page.locator(".preview img").getAttribute("src");

    // Simulate mid-drag: input events only, no change event.
    const slider = page.locator('input[type="range"]').first();
    for (const v of ["70", "50", "40"]) {
      await slider.evaluate((el, val) => {
        el.value = val;
        el.dispatchEvent(new Event("input", { bubbles: true }));
      }, v);
    }
    await expect(page.locator(".opt label").first()).toHaveText("Quality: 40");
    // The engine must NOT have rerun while sliding — same blob URL.
    expect(await page.locator(".preview img").getAttribute("src")).toBe(srcBefore);

    // Releasing the slider (change event) recomputes exactly once.
    await slider.evaluate((el) => {
      el.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await expect
      .poll(async () => page.locator(".preview img").getAttribute("src"), {
        timeout: 15_000,
      })
      .not.toBe(srcBefore);
  });

  test("Clear removes files, thumbnails AND the preview (regression)", async ({ page }) => {
    await page.setInputFiles("#file-in", pngFiles(2));
    await expect(page.locator(".thumb-card")).toHaveCount(2);
    await expect(page.locator(".preview")).toBeVisible({ timeout: 15_000 });
    await page.locator("button", { hasText: "Clear" }).click();
    await expect(page.locator(".thumb-card")).toHaveCount(0);
    await expect(page.locator(".preview")).toHaveCount(0);
  });

  test("compress runs on button click and produces results", async ({ page }) => {
    await page.setInputFiles("#file-in", pngFiles(2));
    await page.locator("button.primary", { hasText: "Compress Image" }).click();
    await expect(page.locator("section.results")).toBeVisible({ timeout: 30_000 });
  });
});
