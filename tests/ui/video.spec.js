// UI tests for the video tools — the ffmpeg.wasm pipeline (ADR-0010).
// These drive the real bundle, so they cover the whole path: the lazy
// /ffmpeg/ load, the worker, argument building in Rust, and the download.
// Inputs are in-memory fixtures: a raw .y4m and a tiny real WebM (the
// stream-copy trim needs an actual compressed container to remux).
import { test, expect } from "@playwright/test";
import { readFile } from "fs/promises";
import { sampleY4m } from "./fixtures/sample-y4m.mjs";
import { sampleWebm } from "./fixtures/sample-webm.mjs";

// First use compiles a ~31 MB wasm; give every test generous room.
test.setTimeout(120_000);

function y4mFile(name = "clip.y4m") {
  return { name, mimeType: "video/x-yuv4mpeg", buffer: sampleY4m() };
}
function webmFile(name = "clip.webm") {
  return { name, mimeType: "video/webm", buffer: sampleWebm() };
}

async function open(page, slug) {
  await page.goto(`/tool/${slug}/`);
  await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
}

async function run(page) {
  await page.locator(".actions button.primary").click();
  await expect(page.locator(".results")).toBeVisible({ timeout: 90_000 });
}

async function downloadBytes(page) {
  const download = page.waitForEvent("download");
  await page.locator(".results button", { hasText: "Download" }).click();
  const dl = await download;
  return { name: dl.suggestedFilename(), bytes: await readFile(await dl.path()) };
}

test.describe("video tools (ffmpeg.wasm)", () => {
  test("video to gif produces a real GIF", async ({ page }) => {
    await open(page, "video-to-gif");
    await page.setInputFiles("#file-in", y4mFile());
    await run(page);
    await expect(page.locator(".results .file-name")).toHaveText("clip.gif");
    const { bytes } = await downloadBytes(page);
    expect(bytes.subarray(0, 6).toString("latin1")).toBe("GIF89a");
    expect(bytes.length).toBeGreaterThan(100);
  });

  test("gif options reach ffmpeg: fps, width and time range", async ({ page }) => {
    await open(page, "video-to-gif");
    await page.setInputFiles("#file-in", y4mFile());
    await page.getByLabel("Frame rate").selectOption("5");
    await page.getByPlaceholder("Width px").fill("16");
    await page.getByLabel("Start time").fill("0.5");
    await page.getByLabel("End time").fill("1.2");
    await run(page);
    const { bytes } = await downloadBytes(page);
    expect(bytes.subarray(0, 6).toString("latin1")).toBe("GIF89a");
    // GIF logical screen width lives at bytes 6-7 (little-endian).
    expect(bytes.readUInt16LE(6)).toBe(16);
  });

  test("trim copies the streams instead of re-encoding", async ({ page }) => {
    await open(page, "trim-video");
    await page.setInputFiles("#file-in", webmFile());
    await page.getByLabel("Start time").fill("0.5");
    await page.getByLabel("End time").fill("1.2");
    await run(page);
    await expect(page.locator(".results .file-name")).toHaveText("clip-trimmed.webm");
    const { bytes } = await downloadBytes(page);
    // Still a real WebM (EBML magic), and smaller than the 1.6 s source.
    expect(bytes.subarray(0, 4)).toEqual(Buffer.from([0x1a, 0x45, 0xdf, 0xa3]));
    expect(bytes.length).toBeLessThan(sampleWebm().length);
    expect(bytes.length).toBeGreaterThan(100);
  });

  test("trim with no times is refused before ffmpeg ever runs", async ({ page }) => {
    await open(page, "trim-video");
    await page.setInputFiles("#file-in", webmFile());
    await page.locator(".actions button.primary").click();
    await expect(page.locator(".error")).toContainText("start time", { timeout: 30_000 });
    await expect(page.locator(".results")).toHaveCount(0);
  });

  test("convert produces MP4 by default and WebM on request", async ({ page }) => {
    await open(page, "convert-video");
    await page.setInputFiles("#file-in", y4mFile());
    await run(page);
    await expect(page.locator(".results .file-name")).toHaveText("clip.mp4");
    const mp4 = await downloadBytes(page);
    expect(mp4.bytes.subarray(4, 8).toString("latin1")).toBe("ftyp");

    await page.getByLabel("Convert to format").selectOption("webm");
    await run(page);
    await expect(page.locator(".results .file-name")).toHaveText("clip.webm");
    const webm = await downloadBytes(page);
    expect(webm.bytes.subarray(0, 4)).toEqual(Buffer.from([0x1a, 0x45, 0xdf, 0xa3]));
  });
});
