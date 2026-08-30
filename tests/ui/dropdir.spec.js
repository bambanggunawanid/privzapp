// Folder drag-and-drop: dropdir.js walks webkitGetAsEntry trees and the
// Rust bridge feeds the normal file list. A real OS drag gesture cannot
// be synthesized headless, so these tests enter through pzIngestEntries —
// the exact function the drop listener calls — with duck-typed entry
// trees, then drive the REAL remaining path: blob URLs → dioxus bridge →
// Rust fetch → file list → engine → download. Only the physical gesture
// itself needs a manual browser pass.
import { test, expect } from "@playwright/test";
import { readFile } from "fs/promises";

// Build a fake FileSystemDirectoryEntry tree in the page. `spec` maps
// relative paths to contents; directories are inferred. The directory
// reader hands out batches of 100 like Chrome does, so the walker's
// batching loop is exercised by any directory with >100 children.
async function ingestTree(page, spec) {
  await page.evaluate((files) => {
    function fileEntry(path, text) {
      return {
        isFile: true,
        isDirectory: false,
        fullPath: "/" + path,
        name: path.split("/").pop(),
        file: (res) => res(new File([text], path.split("/").pop())),
      };
    }
    function dirEntry(path, children) {
      return {
        isFile: false,
        isDirectory: true,
        fullPath: "/" + path,
        name: path.split("/").pop(),
        createReader: () => {
          let given = 0;
          return {
            readEntries: (res) => {
              const batch = children.slice(given, given + 100);
              given += batch.length;
              res(batch);
            },
          };
        },
      };
    }
    const dirs = new Map();
    const roots = [];
    function dirFor(path) {
      if (path === "") return { children: roots };
      if (!dirs.has(path)) {
        const d = { children: [] };
        dirs.set(path, d);
        const parent = dirFor(path.split("/").slice(0, -1).join("/"));
        d.entry = dirEntry(path, d.children);
        parent.children.push(d.entry);
      }
      return dirs.get(path);
    }
    for (const [path, text] of Object.entries(files)) {
      dirFor(path.split("/").slice(0, -1).join("/")).children.push(
        fileEntry(path, text),
      );
    }
    return pzIngestEntries(roots);
  }, spec);
}

test.describe("folder drag-and-drop", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tool/zip-files/");
    await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
  });

  test("a dropped folder tree lands in the file list with relative paths", async ({ page }) => {
    const spec = { "notes/readme.txt": "hello" };
    // 120 files in one directory: proves the walker keeps calling
    // readEntries past the first 100-entry batch.
    for (let i = 0; i < 120; i++) spec[`photos/img-${String(i).padStart(3, "0")}.txt`] = `p${i}`;
    spec["photos/raw/deep.txt"] = "nested";
    spec["photos/.DS_Store"] = "junk";
    spec["photos/Thumbs.db"] = "junk";
    await ingestTree(page, spec);

    // 120 + readme + deep; the OS clutter files are dropped.
    await expect(page.locator(".file-name")).toHaveCount(122, { timeout: 20_000 });
    await expect(page.locator(".file-name", { hasText: "photos/raw/deep.txt" })).toBeVisible();
    await expect(page.locator(".file-name", { hasText: ".DS_Store" })).toHaveCount(0);

    await page.locator(".actions button.primary").click();
    await expect(page.locator(".results")).toBeVisible({ timeout: 60_000 });
    const download = page.waitForEvent("download");
    await page.locator(".results button", { hasText: "Download" }).click();
    const bytes = await readFile(await (await download).path());
    // Real zip, and the folder structure survived into it: zip headers
    // store the entry names uncompressed.
    expect(bytes.subarray(0, 4)).toEqual(Buffer.from([0x50, 0x4b, 0x03, 0x04]));
    expect(bytes.includes(Buffer.from("photos/raw/deep.txt"))).toBe(true);
    expect(bytes.includes(Buffer.from("notes/readme.txt"))).toBe(true);
  });

  test("a second folder drop appends instead of replacing", async ({ page }) => {
    await ingestTree(page, { "a/one.txt": "1" });
    await expect(page.locator(".file-name")).toHaveCount(1, { timeout: 20_000 });
    await ingestTree(page, { "b/two.txt": "2" });
    await expect(page.locator(".file-name")).toHaveCount(2, { timeout: 20_000 });
  });
});
