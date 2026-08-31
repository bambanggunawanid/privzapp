// Folder drag-and-drop, driven by REAL drags.
//
// This was originally written against `pzIngestEntries` with fake entry
// objects, on the belief that a genuine directory drop couldn't be
// synthesized — `webkitGetAsEntry()` does return null for
// programmatically built DataTransferItems. That's true of the standard
// API, but Chrome DevTools Protocol's `Input.dispatchDragEvent` accepts
// real filesystem paths and Chrome builds a real entry tree from them.
// So these drop actual folders from disk and exercise the whole path:
// the OS-level drop, the entry walker, blob URLs, the Rust bridge, the
// engine, and the downloaded archive.
import { test, expect } from "@playwright/test";
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";

let root; // temp dir holding the fixture trees

test.beforeAll(() => {
  root = mkdtempSync(join(tmpdir(), "pz-drop-"));
  const holiday = join(root, "holiday");
  const photos = join(holiday, "photos");
  const raw = join(photos, "raw");
  mkdirSync(raw, { recursive: true });
  writeFileSync(join(holiday, "readme.txt"), "hello");
  writeFileSync(join(raw, "deep.txt"), "nested");
  // Chrome's readEntries() returns at most 100 entries per call, so a
  // directory with more than that proves the walker keeps looping.
  for (let i = 1; i <= 120; i++) {
    writeFileSync(join(photos, `img-${String(i).padStart(3, "0")}.txt`), `p${i}`);
  }
  // Desktop clutter that must never reach the file list.
  for (const junk of [".DS_Store", "Thumbs.db", "desktop.ini"]) {
    writeFileSync(join(photos, junk), "junk");
  }
  // A second folder, for the "appends" case.
  mkdirSync(join(root, "extra"), { recursive: true });
  writeFileSync(join(root, "extra", "one.txt"), "1");
  // Loose files, for the plain-drop case.
  writeFileSync(join(root, "loose-a.txt"), "a");
  writeFileSync(join(root, "loose-b.txt"), "b");
});

test.afterAll(() => rmSync(root, { recursive: true, force: true }));

// Drop real paths onto the page's dropzone, the way a file manager would.
async function dropPaths(page, paths, selector = ".dropzone") {
  const cdp = await page.context().newCDPSession(page);
  const box = await page.locator(selector).boundingBox();
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  const data = { items: [], files: paths, dragOperationsMask: 1 };
  for (const type of ["dragEnter", "dragOver", "drop"]) {
    await cdp.send("Input.dispatchDragEvent", { type, x, y, data });
  }
  await cdp.detach();
}

async function openTool(page, slug) {
  await page.goto(`/tool/${slug}/`);
  await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
}

test.describe("folder drag-and-drop (real drops)", () => {
  test("a dropped folder arrives whole, with relative paths and no clutter", async ({ page }) => {
    await openTool(page, "zip-files");
    await dropPaths(page, [join(root, "holiday")]);

    // 120 photos + readme + deep.txt; the three clutter files are skipped.
    await expect(page.locator(".file-name")).toHaveCount(122, { timeout: 30_000 });
    const names = await page.locator(".file-name").allTextContents();
    expect(names).toContain("holiday/photos/raw/deep.txt");
    expect(names).toContain("holiday/readme.txt");
    // Proves readEntries kept being called past the first 100-entry batch.
    expect(names.filter((n) => n.includes("/photos/img-")).length).toBe(120);
    for (const junk of [".DS_Store", "Thumbs.db", "desktop.ini"]) {
      expect(names.some((n) => n.endsWith(junk))).toBe(false);
    }
  });

  test("the archive keeps the folder structure", async ({ page }) => {
    await openTool(page, "zip-files");
    await dropPaths(page, [join(root, "holiday")]);
    await expect(page.locator(".file-name")).toHaveCount(122, { timeout: 30_000 });

    await page.locator(".actions button.primary").click();
    await expect(page.locator(".results")).toBeVisible({ timeout: 60_000 });
    const download = page.waitForEvent("download");
    await page.locator(".results button", { hasText: "Download" }).click();
    const zip = readFileSync(await (await download).path());

    expect(zip.subarray(0, 4)).toEqual(Buffer.from([0x50, 0x4b, 0x03, 0x04]));
    // Entry names are stored uncompressed, so the paths are greppable.
    expect(zip.includes(Buffer.from("holiday/photos/raw/deep.txt"))).toBe(true);
    expect(zip.includes(Buffer.from("holiday/readme.txt"))).toBe(true);
  });

  test("plain file drops still go through the normal handler", async ({ page }) => {
    await openTool(page, "zip-files");
    await dropPaths(page, [join(root, "loose-a.txt"), join(root, "loose-b.txt")]);
    await expect(page.locator(".file-name")).toHaveCount(2, { timeout: 30_000 });
    const names = await page.locator(".file-name").allTextContents();
    // No directory walked, so no relative paths — just the file names.
    expect(names.sort()).toEqual(["loose-a.txt", "loose-b.txt"]);
  });

  test("a folder and loose files can be dropped together", async ({ page }) => {
    await openTool(page, "zip-files");
    await dropPaths(page, [join(root, "extra"), join(root, "loose-a.txt")]);
    await expect(page.locator(".file-name")).toHaveCount(2, { timeout: 30_000 });
    const names = await page.locator(".file-name").allTextContents();
    expect(names).toContain("extra/one.txt");
    expect(names).toContain("loose-a.txt");
  });

  test("a second folder drop appends instead of replacing", async ({ page }) => {
    await openTool(page, "zip-files");
    await dropPaths(page, [join(root, "extra")]);
    await expect(page.locator(".file-name")).toHaveCount(1, { timeout: 30_000 });
    await dropPaths(page, [join(root, "loose-b.txt")]);
    await expect(page.locator(".file-name")).toHaveCount(2, { timeout: 30_000 });
  });

  test("single-file tools and the editor never swallow a folder drop", async ({ page }) => {
    // Extract ZIP is single-file: the folder interceptor must stay inert
    // there, or files would be queued with nothing to consume them.
    await openTool(page, "unzip");
    await dropPaths(page, [join(root, "extra")]);
    await page.waitForTimeout(1_500);
    await expect(page.locator(".file-name")).toHaveCount(0);

    await page.goto("/tool/edit-pdf/");
    await page.waitForSelector("#pdf-in", { state: "attached", timeout: 45_000 });
    await dropPaths(page, [join(root, "extra")]);
    await page.waitForTimeout(1_500);
    await expect(page.locator(".pz-page")).toHaveCount(0);

    // And a multi-file tool visited afterwards must not inherit them.
    await openTool(page, "zip-files");
    await page.waitForTimeout(1_500);
    await expect(page.locator(".file-name")).toHaveCount(0);
  });
});
