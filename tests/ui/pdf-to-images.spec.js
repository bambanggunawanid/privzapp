// UI tests for the PDF to Image tool — the one tool whose pages are
// rasterized by the browser (PDF.js) rather than the Rust engine
// (ADR-0009). These drive the real wasm bundle, so they cover the whole
// path: render in JS → name and package in Rust → download.
import { test, expect } from "@playwright/test";
import { readFile } from "fs/promises";
import { samplePdf } from "./fixtures/sample-pdf.mjs";

// The fixture is a two-page PDF.
function pdfFile(name = "sample.pdf") {
  return { name, mimeType: "application/pdf", buffer: samplePdf() };
}

async function runTool(page) {
  await page.locator(".actions button.primary").click();
  await expect(page.locator(".results")).toBeVisible({ timeout: 30_000 });
}

async function downloadBytes(page) {
  const download = page.waitForEvent("download");
  await page.locator(".results button", { hasText: "Download" }).click();
  const dl = await download;
  return { name: dl.suggestedFilename(), bytes: await readFile(await dl.path()) };
}

test.describe("pdf to image", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tool/pdf-to-images/");
    await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
  });

  test("every page becomes an image, zipped when there is more than one", async ({ page }) => {
    await page.setInputFiles("#file-in", pdfFile());
    await runTool(page);
    // Two pages in → one zip out, named after the source file.
    await expect(page.locator(".results .file-name")).toHaveCount(1);
    await expect(page.locator(".results .file-name")).toHaveText("sample-pages.zip");

    const { name, bytes } = await downloadBytes(page);
    expect(name).toBe("sample-pages.zip");
    // Real zip: local file header magic, and big enough to hold 2 PNGs.
    expect(bytes.subarray(0, 4)).toEqual(Buffer.from([0x50, 0x4b, 0x03, 0x04]));
    expect(bytes.length).toBeGreaterThan(1000);
  });

  test("a single page downloads as the image itself, not a zip", async ({ page }) => {
    await page.setInputFiles("#file-in", pdfFile());
    await page.getByPlaceholder("1-3,5").fill("1");
    await runTool(page);
    await expect(page.locator(".results .file-name")).toHaveText("sample-page-01.png");

    const { name, bytes } = await downloadBytes(page);
    expect(name).toBe("sample-page-01.png");
    // Real PNG signature — proves the page actually rasterized.
    expect(bytes.subarray(0, 8)).toEqual(
      Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    );
  });

  test("format and resolution options reach the renderer", async ({ page }) => {
    await page.setInputFiles("#file-in", pdfFile());
    await page.getByLabel("Image format").selectOption("jpg");
    await page.getByLabel("Resolution").selectOption("1");
    await page.getByPlaceholder("1-3,5").fill("2");
    await runTool(page);
    await expect(page.locator(".results .file-name")).toHaveText("sample-page-02.jpg");

    const { bytes } = await downloadBytes(page);
    // JPEG SOI marker, so the format select really changed the encoding.
    expect(bytes.subarray(0, 2)).toEqual(Buffer.from([0xff, 0xd8]));
  });

  test("a page outside the document is reported, not silently dropped", async ({ page }) => {
    await page.setInputFiles("#file-in", pdfFile());
    await page.getByPlaceholder("1-3,5").fill("9");
    await page.locator(".actions button.primary").click();
    // The real bound check happens in JS, against the actual page count.
    await expect(page.locator(".error")).toContainText(/out of range|1-2/, {
      timeout: 30_000,
    });
    await expect(page.locator(".results")).toHaveCount(0);
  });
});
