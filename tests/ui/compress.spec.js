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
    const srcs = await page.locator(".thumb-card .thumb-img").evaluateAll((imgs) =>
      imgs.map((i) => i.src),
    );
    expect(srcs).toHaveLength(3);
    for (const src of srcs) expect(src).toMatch(/^blob:/);
    // Before/after comparison panes.
    await expect(page.locator(".preview-before")).toBeVisible({ timeout: 15_000 });
    await expect(page.locator(".preview-after")).toBeVisible({ timeout: 15_000 });
  });

  test("clicking a thumbnail switches the preview to that image", async ({ page }) => {
    await page.setInputFiles("#file-in", pngFiles(3));
    await expect(page.locator(".preview p")).toContainText("photo-1", { timeout: 15_000 });
    await expect(page.locator(".thumb-card").first()).toHaveClass(/selected/);
    await page.locator(".thumb-card").nth(1).locator(".thumb-img").click();
    await expect(page.locator(".preview p")).toContainText("photo-2", { timeout: 15_000 });
    await expect(page.locator(".thumb-card").nth(1)).toHaveClass(/selected/);
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
    const srcBefore = await page.locator(".preview-after").getAttribute("src");

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
    expect(await page.locator(".preview-after").getAttribute("src")).toBe(srcBefore);

    // Releasing the slider (change event) recomputes exactly once.
    await slider.evaluate((el) => {
      el.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await expect
      .poll(async () => page.locator(".preview-after").getAttribute("src"), {
        timeout: 15_000,
      })
      .not.toBe(srcBefore);
  });

  test("resolution percent control shrinks output and steps by 10", async ({ page }) => {
    await page.setInputFiles("#file-in", pngFiles(1));
    await expect(page.locator(".preview-after")).toBeVisible({ timeout: 15_000 });
    await expect(page.locator(".opt label").nth(1)).toHaveText(
      "Resolution: 100% of original",
    );
    await page.locator('button[title="Resolution −10%"]').click();
    await expect(page.locator(".opt label").nth(1)).toHaveText(
      "Resolution: 90% of original",
    );
    // Dropping to 50% via the slider recomputes on release.
    const srcBefore = await page.locator(".preview-after").getAttribute("src");
    const slider = page.locator('input[type="range"]').nth(1);
    await slider.evaluate((el) => {
      el.value = 50;
      el.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await expect(page.locator(".opt label").nth(1)).toHaveText(
      "Resolution: 50% of original",
    );
    await expect
      .poll(async () => page.locator(".preview-after").getAttribute("src"), {
        timeout: 15_000,
      })
      .not.toBe(srcBefore);
  });

  test("switching thumbnails reuses cached previews (no recompress)", async ({ page }) => {
    await page.setInputFiles("#file-in", pngFiles(2));
    await expect(page.locator(".preview p")).toContainText("photo-1", { timeout: 15_000 });
    const src1 = await page.locator(".preview-after").getAttribute("src");
    await page.locator(".thumb-card").nth(1).locator(".thumb-img").click();
    await expect(page.locator(".preview p")).toContainText("photo-2", { timeout: 15_000 });
    // Back to the first image: the cache must hand back the SAME blob URL
    // instantly instead of recompressing.
    await page.locator(".thumb-card").nth(0).locator(".thumb-img").click();
    await expect(page.locator(".preview p")).toContainText("photo-1");
    expect(await page.locator(".preview-after").getAttribute("src")).toBe(src1);
  });

  test("convert image also offers the resolution control", async ({ page }) => {
    await page.goto("/tool/convert-img/");
    await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
    await page.setInputFiles("#file-in", pngFiles(1));
    await expect(page.locator(".opt label", { hasText: "Resolution" })).toHaveText(
      "Resolution: 100% of original",
    );
    await page.locator('button[title="Resolution −10%"]').click();
    await expect(page.locator(".opt label", { hasText: "Resolution" })).toHaveText(
      "Resolution: 90% of original",
    );
    await expect(page.locator(".preview-after")).toBeVisible({ timeout: 15_000 });
  });

  test("all image tools show the live preview (grayscale, flip)", async ({ page }) => {
    // Grayscale: preview appears right after upload.
    await page.goto("/tool/grayscale-img/");
    await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
    await page.setInputFiles("#file-in", pngFiles(1));
    await expect(page.locator(".preview-after")).toBeVisible({ timeout: 15_000 });

    // Flip: changing the direction recomputes the preview.
    await page.goto("/tool/flip-img/");
    await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
    await page.setInputFiles("#file-in", pngFiles(1));
    await expect(page.locator(".preview-after")).toBeVisible({ timeout: 15_000 });
    const srcBefore = await page.locator(".preview-after").getAttribute("src");
    await page.locator("select").selectOption("vertical");
    await expect
      .poll(async () => page.locator(".preview-after").getAttribute("src"), {
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
