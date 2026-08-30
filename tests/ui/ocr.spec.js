// UI tests for the OCR tools — the tesseract-wasm pipeline (ADR-0011).
// The text image is generated on a canvas inside the test browser, so
// the fixture is deterministic and no binary lives in the repo; the
// scanned-PDF case rides the existing sample PDF through the ADR-0009
// rasterizer first.
import { test, expect } from "@playwright/test";
import { readFile } from "fs/promises";
import { samplePdf } from "./fixtures/sample-pdf.mjs";

test.setTimeout(120_000);

// Render `lines` of text onto a white canvas and return PNG bytes.
async function textPng(page, lines) {
  const b64 = await page.evaluate(async (lines) => {
    const canvas = document.createElement("canvas");
    canvas.width = 640;
    canvas.height = 60 + lines.length * 64;
    const c = canvas.getContext("2d");
    c.fillStyle = "#fff";
    c.fillRect(0, 0, canvas.width, canvas.height);
    c.fillStyle = "#000";
    c.font = "bold 44px sans-serif";
    lines.forEach((l, i) => c.fillText(l, 24, 72 + i * 64));
    const blob = await new Promise((r) => canvas.toBlob(r, "image/png"));
    const fr = new FileReader();
    return new Promise((r) => {
      fr.onload = () => r(fr.result.split(",", 2)[1]);
      fr.readAsDataURL(blob);
    });
  }, lines);
  return Buffer.from(b64, "base64");
}

async function run(page) {
  await page.locator(".actions button.primary").click();
  await expect(page.locator(".results")).toBeVisible({ timeout: 90_000 });
}

async function downloadNth(page, i) {
  const download = page.waitForEvent("download");
  await page.locator(".results button", { hasText: "Download" }).nth(i).click();
  const dl = await download;
  return { name: dl.suggestedFilename(), text: (await readFile(await dl.path())).toString("utf8") };
}

test.describe("OCR tools (tesseract-wasm)", () => {
  test("image to text reads a picture back out, one .txt per image", async ({ page }) => {
    await page.goto("/tool/image-to-text/");
    await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
    const one = await textPng(page, ["Invoice 4207 due Friday"]);
    const two = await textPng(page, ["Meeting moved to Room 12"]);
    await page.setInputFiles("#file-in", [
      { name: "invoice.png", mimeType: "image/png", buffer: one },
      { name: "note.png", mimeType: "image/png", buffer: two },
    ]);
    await run(page);
    await expect(page.locator(".results .file-name")).toHaveCount(2);

    const first = await downloadNth(page, 0);
    expect(first.name).toBe("invoice.txt");
    expect(first.text).toContain("Invoice 4207 due Friday");
    const second = await downloadNth(page, 1);
    expect(second.name).toBe("note.txt");
    expect(second.text).toContain("Meeting moved to Room 12");
  });

  test("language picker offers the staged models", async ({ page }) => {
    await page.goto("/tool/image-to-text/");
    await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
    const values = await page
      .getByLabel("Recognition language")
      .locator("option")
      .evaluateAll((os) => os.map((o) => o.value));
    expect(values).toEqual(["eng", "ind"]);
  });

  test("OCR PDF rasterizes the pages and reads them, with page markers", async ({ page }) => {
    await page.goto("/tool/ocr-pdf/");
    await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
    await page.setInputFiles("#file-in", {
      name: "scan.pdf",
      mimeType: "application/pdf",
      buffer: samplePdf(),
    });
    await run(page);
    await expect(page.locator(".results .file-name")).toHaveText("scan.txt");
    const { text } = await downloadNth(page, 0);
    // The fixture's two pages carry real rendered text.
    expect(text).toMatch(/Hello/i);
    expect(text).toMatch(/Second page/i);
    expect(text).toContain("----- Page 1 -----");
    expect(text).toContain("----- Page 2 -----");
  });

  test("OCR PDF respects the page range", async ({ page }) => {
    await page.goto("/tool/ocr-pdf/");
    await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
    await page.setInputFiles("#file-in", {
      name: "scan.pdf",
      mimeType: "application/pdf",
      buffer: samplePdf(),
    });
    await page.getByPlaceholder("1-3,5").fill("2");
    await run(page);
    const { text } = await downloadNth(page, 0);
    expect(text).toMatch(/Second page/i);
    expect(text).not.toMatch(/Hello PrivZapp/i);
    // A single page gets no page markers.
    expect(text).not.toContain("----- Page");
  });
});
