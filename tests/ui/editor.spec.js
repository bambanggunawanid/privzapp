// UI tests for the PDF editor workspace. Each test pins a behavior the
// owner reported broken at least once — keep them green.
import { readFileSync } from "node:fs";
import { test, expect } from "@playwright/test";
import { samplePdf } from "./fixtures/sample-pdf.mjs";
import { samplePng } from "./fixtures/sample-png.mjs";

async function gotoEditor(page) {
  await page.goto("/tool/edit-pdf/");
  // Wasm app booted once the hidden file input is mounted.
  await page.waitForSelector("#pdf-in", { state: "attached", timeout: 45_000 });
}

async function openPdf(page) {
  await page.setInputFiles("#pdf-in", {
    name: "sample.pdf",
    mimeType: "application/pdf",
    buffer: samplePdf(),
  });
  await expect(page.locator(".pz-page")).toHaveCount(2, { timeout: 30_000 });
  await expect(page.locator(".pz-thumb")).toHaveCount(2);
}

async function pageBox(page) {
  return await page.locator(".pz-page").first().boundingBox();
}

test.describe("editor workspace", () => {
  test.beforeEach(async ({ page }) => {
    await gotoEditor(page);
  });

  test("default tool is the cursor, not the pen", async ({ page }) => {
    await expect(page.locator('button[title^="Cursor"]')).toHaveClass(/active/);
    await expect(page.locator('button[title^="Pen"]')).not.toHaveClass(/active/);
    const mode = await page.evaluate(() => window.pzEd.tool.mode);
    expect(mode).toBe("cursor");
  });

  test("loads a PDF: pages, thumbnails, page indicator", async ({ page }) => {
    await openPdf(page);
    await expect(page.locator("#pz-pageno")).toHaveText("1 / 2");
    await expect(page.locator(".pz-thumb").first()).toHaveClass(/active/);
  });

  test("append PDF stacks the imported pages below the last page", async ({ page }) => {
    await openPdf(page);
    // The rail offers a visible tile wired to the shared #append-in input.
    await expect(page.locator(".ed-addpdf")).toBeVisible();
    await page.setInputFiles("#append-in", {
      name: "second.pdf",
      mimeType: "application/pdf",
      buffer: samplePdf(),
    });
    // 2 + 2 pages, original first page still on top.
    await expect(page.locator(".pz-page")).toHaveCount(4, { timeout: 30_000 });
    await expect(page.locator(".pz-thumb")).toHaveCount(4);
    await expect(page.locator("#pz-pageno")).toHaveText("1 / 4");
    await expect(page.locator(".pz-page").first().locator(".pz-textlayer"))
      .toContainText("Hello PrivZapp", { timeout: 15_000 });
  });

  test("text tool activates (regression: used to stay on pen/cursor)", async ({ page }) => {
    await openPdf(page);
    await page.locator('button[title^="Text"]').click();
    await expect(page.locator('button[title^="Text"]')).toHaveClass(/active/);
    await expect(page.locator('button[title^="Cursor"]')).not.toHaveClass(/active/);
    const mode = await page.evaluate(() => window.pzEd.tool.mode);
    expect(mode).toBe("text");
  });

  test("text tool places an editable box anywhere; box stays editable after placing", async ({ page }) => {
    await openPdf(page);
    await page.locator('button[title^="Text"]').click();
    const box = await pageBox(page);
    // Click an empty area (below the fixture's headline text).
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    const textBox = page.locator(".pz-text").first();
    await expect(textBox).toBeVisible();
    await expect(textBox).toBeFocused();
    await page.keyboard.type("Hello test");
    await expect(textBox).toHaveText("Hello test");

    // Click away, then come back with the cursor tool and change it —
    // regression: text used to be permanent once placed.
    await page.mouse.click(box.x + 30, box.y + box.height - 40);
    await page.locator('button[title^="Cursor"]').click();
    await textBox.click();
    await expect(textBox).toBeFocused();
    await page.keyboard.press("End");
    await page.keyboard.type(" edited");
    await expect(textBox).toHaveText("Hello test edited");
  });

  test("cursor tool moves a placed text box by dragging", async ({ page }) => {
    await openPdf(page);
    await page.locator('button[title^="Text"]').click();
    const box = await pageBox(page);
    await page.mouse.click(box.x + 200, box.y + 400);
    await page.keyboard.type("Move me");
    await page.locator('button[title^="Cursor"]').click();
    const before = await page.evaluate(() => window.pzEd.texts[1][0].x);
    const textBox = page.locator(".pz-text").first();
    const tb = await textBox.boundingBox();
    await page.mouse.move(tb.x + 5, tb.y + 5);
    await page.mouse.down();
    await page.mouse.move(tb.x + 105, tb.y + 55, { steps: 5 });
    await page.mouse.up();
    const after = await page.evaluate(() => window.pzEd.texts[1][0].x);
    expect(after).toBeGreaterThan(before + 50);
  });

  test("clicking detected PDF text converts it to an editable box (retype)", async ({ page }) => {
    await openPdf(page);
    const span = page.locator('.pz-textlayer span', { hasText: "Hello PrivZapp" }).first();
    await span.click();
    const textBox = page.locator(".pz-text").first();
    await expect(textBox).toBeVisible();
    await expect(textBox).toHaveText("Hello PrivZapp");
    await expect(textBox).toBeFocused();
    // A white-out rect now covers the original.
    const rects = await page.evaluate(() => (window.pzEd.rects[1] || []).length);
    expect(rects).toBe(1);
    // Style inheritance (regression: retype used to reset the style):
    // the fixture text is regular black, so the box must be near-black
    // and normal weight — not restyled.
    const style = await textBox.evaluate((el) => {
      const s = getComputedStyle(el);
      return { color: s.color, weight: s.fontWeight };
    });
    const rgb = style.color.match(/\d+/g).map(Number);
    for (const ch of rgb.slice(0, 3)) expect(ch).toBeLessThan(110);
    expect(["400", "normal"]).toContain(style.weight);
  });

  test("editing existing PDF text keeps the original font (no Helvetica swap)", async ({ page }) => {
    await openPdf(page);
    const span = page.locator(".pz-textlayer span", { hasText: "Hello PrivZapp" }).first();
    await span.click();
    const box = page.locator(".pz-text").first();
    await expect(box).toBeFocused();
    await box.press("ControlOrMeta+a");
    await box.pressSequentially("Edited PrivZapp");
    // Export bakes the pending edit. The downloaded PDF must be rewritten
    // through the run's ORIGINAL font resource — the Helvetica fallback
    // (resource "PZtx") appearing means the style-changing swap is back.
    await page.locator("button", { hasText: "Export ↓" }).click();
    const download = page.waitForEvent("download");
    await page.locator("button", { hasText: "⬇ Download PDF" }).click();
    // Resource names live in the (always-plain) page dictionaries, so this
    // scan is reliable; content-stream literals are NOT (lopdf may deflate
    // the rebuilt stream), which is why the text itself is asserted through
    // PDF.js below instead of raw bytes.
    const bytes = readFileSync(await (await download).path());
    expect(bytes.includes("PZtx")).toBe(false);
    // The bake re-renders the mutated document: the old text is gone
    // from PDF.js's own text layer, not just covered.
    await expect(page.locator(".pz-textlayer").first()).toContainText("Edited PrivZapp", {
      timeout: 30_000,
    });
    await expect(page.locator(".pz-textlayer").first()).not.toContainText("Hello");
  });

  test("inserted image is a live object: lands at source size, drags, resizes, deletes", async ({ page }) => {
    await openPdf(page);
    await page.setInputFiles("#img-in", {
      name: "stamp.png",
      mimeType: "image/png",
      buffer: samplePng(),
    });
    // Placed immediately — no rectangle-drawing step.
    const img = page.locator(".pz-img").first();
    await expect(img).toBeVisible({ timeout: 15_000 });
    const rec = await page.evaluate(() => {
      const r = window.pzEd.images[1][0];
      return { x: r.x, w: r.w, h: r.h, opacity: r.opacity };
    });
    expect(rec.w).toBeGreaterThan(16);
    expect(rec.opacity).toBe(1);

    // Drag moves it.
    const bb = await img.boundingBox();
    await page.mouse.move(bb.x + bb.width / 2, bb.y + bb.height / 2);
    await page.mouse.down();
    await page.mouse.move(bb.x + bb.width / 2 + 90, bb.y + bb.height / 2 + 40, { steps: 5 });
    await page.mouse.up();
    const movedX = await page.evaluate(() => window.pzEd.images[1][0].x);
    expect(movedX).toBeGreaterThan(rec.x + 50);

    // Per-object opacity.
    await page.locator(".pz-obj-opacity").evaluate((el) => {
      el.value = 50;
      el.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(await page.evaluate(() => window.pzEd.images[1][0].opacity)).toBeCloseTo(0.5, 5);

    // ✕ deletes; Ctrl+Z brings it back.
    await img.hover();
    await page.locator(".pz-obj-del").click();
    await expect(page.locator(".pz-img")).toHaveCount(0);
    await page.locator(".ed-canvas").click({ position: { x: 30, y: 30 } });
    await page.keyboard.press("Control+z");
    await expect(page.locator(".pz-img")).toHaveCount(1);
  });

  test("image resize is free by default, proportional with Shift", async ({ page }) => {
    await openPdf(page);
    await page.setInputFiles("#img-in", {
      name: "stamp.png",
      mimeType: "image/png",
      buffer: samplePng(),
    });
    const img = page.locator(".pz-img").first();
    await expect(img).toBeVisible({ timeout: 15_000 });
    const orig = await page.evaluate(() => {
      const r = window.pzEd.images[1][0];
      return { w: r.w, h: r.h };
    });

    const dragHandle = async (dx, dy) => {
      await img.hover();
      const hb = await page.locator(".pz-obj-resize").boundingBox();
      await page.mouse.move(hb.x + hb.width / 2, hb.y + hb.height / 2);
      await page.mouse.down();
      await page.mouse.move(hb.x + dx, hb.y + dy, { steps: 4 });
      await page.mouse.up();
    };

    // Free resize: stretch mostly horizontally → ratio changes.
    await dragHandle(80, 8);
    const free = await page.evaluate(() => {
      const r = window.pzEd.images[1][0];
      return { w: r.w, h: r.h };
    });
    expect(free.w / free.h).toBeGreaterThan((orig.w / orig.h) * 1.2);

    // Shift resize: same drag keeps the aspect ratio.
    await page.keyboard.down("Shift");
    await dragHandle(60, 5);
    await page.keyboard.up("Shift");
    const prop = await page.evaluate(() => {
      const r = window.pzEd.images[1][0];
      return { w: r.w, h: r.h };
    });
    expect(prop.w / prop.h).toBeCloseTo(free.w / free.h, 1);
    expect(prop.w).toBeGreaterThan(free.w);
  });

  test("highlighter draws translucent yellow strokes", async ({ page }) => {
    await openPdf(page);
    await page.locator('button[title^="Highlighter"]').click();
    const box = await pageBox(page);
    await page.mouse.move(box.x + 100, box.y + 120);
    await page.mouse.down();
    await page.mouse.move(box.x + 300, box.y + 122, { steps: 8 });
    await page.mouse.up();
    const stroke = await page.evaluate(() => window.pzEd.strokes[1][0]);
    expect(stroke.opacity).toBeCloseTo(0.4, 5);
    expect(stroke.color).toBe("#ffe600");
    expect(stroke.points.length).toBeGreaterThan(2);
  });

  test("Ctrl+Z undoes a stroke, Ctrl+Shift+Z redoes it", async ({ page }) => {
    await openPdf(page);
    await page.locator('button[title^="Pen"]').click();
    const box = await pageBox(page);
    await page.mouse.move(box.x + 100, box.y + 300);
    await page.mouse.down();
    await page.mouse.move(box.x + 200, box.y + 350, { steps: 5 });
    await page.mouse.up();
    const count = () => page.evaluate(() => (window.pzEd.strokes[1] || []).length);
    expect(await count()).toBe(1);
    await page.keyboard.press("Control+z");
    expect(await count()).toBe(0);
    await page.keyboard.press("Control+Shift+z");
    expect(await count()).toBe(1);
  });

  test("undo/redo buttons in the top bar work", async ({ page }) => {
    await openPdf(page);
    await page.locator('button[title^="Pen"]').click();
    const box = await pageBox(page);
    await page.mouse.move(box.x + 120, box.y + 500);
    await page.mouse.down();
    await page.mouse.move(box.x + 220, box.y + 520, { steps: 4 });
    await page.mouse.up();
    await page.locator('button[title^="Undo"]').click();
    expect(await page.evaluate(() => (window.pzEd.strokes[1] || []).length)).toBe(0);
    await page.locator('button[title^="Redo"]').click();
    expect(await page.evaluate(() => (window.pzEd.strokes[1] || []).length)).toBe(1);
  });

  test("zoom controls re-render and keep the zoom level display in sync", async ({ page }) => {
    await openPdf(page);
    const before = (await pageBox(page)).width;
    await page.locator('button[title="Zoom in"]').click();
    await expect(page.locator("#pz-zoomlvl")).toHaveText("125%", { timeout: 15_000 });
    await expect
      .poll(async () => (await pageBox(page)).width, { timeout: 15_000 })
      .toBeGreaterThan(before * 1.1);
    await page.locator('button[title="Fit width"]').click();
    await expect(page.locator("#pz-zoomlvl")).toHaveText("100%", { timeout: 15_000 });
  });

  test("ruler and grid views toggle", async ({ page }) => {
    await openPdf(page);
    await page.locator('button[title^="Toggle rulers"]').click();
    await expect(page.locator(".ed-canvas-wrap")).toHaveClass(/ed-rulers/);
    await expect(page.locator("#pz-ruler-h")).toBeVisible();
    await page.locator('button[title^="Toggle grid"]').click();
    await expect(page.locator("#pz-pages")).toHaveClass(/pz-grid-on/);
    await page.locator('button[title^="Toggle rulers"]').click();
    await expect(page.locator(".ed-canvas-wrap")).not.toHaveClass(/ed-rulers/);
  });

  test("dragging a thumbnail reorders the pages through the engine", async ({ page }) => {
    await openPdf(page);
    await page.dragAndDrop(
      '.pz-thumb[data-page="1"]',
      '.pz-thumb[data-page="2"]',
      { targetPosition: { x: 40, y: 100 } },
    );
    // The reorder runs through the Rust engine and re-renders.
    await expect(page.locator(".ed-right .notice")).toHaveText(/reorganized/i, {
      timeout: 30_000,
    });
    await expect(page.locator(".pz-thumb")).toHaveCount(2);
  });

  test("redaction really removes the text, not just covers it", async ({ page }) => {
    await openPdf(page);
    await expect(page.locator(".pz-textlayer").first()).toContainText("Hello PrivZapp", {
      timeout: 15_000,
    });
    // Drag a redaction box over the headline.
    await page.locator('button[title^="Redact"]').click();
    const span = page.locator(".pz-textlayer span", { hasText: "Hello PrivZapp" }).first();
    const sb = await span.boundingBox();
    await page.mouse.move(sb.x - 8, sb.y - 8);
    await page.mouse.down();
    await page.mouse.move(sb.x + sb.width + 8, sb.y + sb.height + 8, { steps: 4 });
    await page.mouse.up();
    await expect(page.locator(".pz-redact")).toHaveCount(1);
    // Any document operation bakes pending edits — after the re-render,
    // PDF.js's own text layer must no longer contain the words at all.
    await page.locator("button", { hasText: "Add page numbers" }).click();
    await expect(page.locator(".ed-busybar")).toHaveCount(0, { timeout: 30_000 });
    await expect(page.locator(".pz-page").first()).toBeVisible({ timeout: 30_000 });
    await expect(page.locator(".pz-textlayer").first()).not.toContainText("Hello", {
      timeout: 15_000,
    });
    // The second page's text is untouched.
    await expect(page.locator(".pz-textlayer").nth(1)).toContainText("Second page");
  });

  test("export pages as PNG downloads a zip of rendered pages", async ({ page }) => {
    await openPdf(page);
    await page.locator("button", { hasText: "⬇ Export" }).click();
    const download = page.waitForEvent("download");
    await page.locator("button", { hasText: "Pages as PNG" }).click();
    expect((await download).suggestedFilename()).toBe("sample-pages.zip");
  });

  // Regression: uploading a password-protected PDF dumped the raw eval
  // failure ("Communication(...PasswordException...)") on the user, and
  // rendered it twice. A protected PDF is a normal thing to try.
  test("a password-protected PDF is refused with advice, not internals", async ({ page }) => {
    await gotoEditor(page);
    // Build one with the app's own Protect PDF tool.
    const enc = await page.evaluate(async () => null); // keep lint quiet
    void enc;
    await page.goto("/tool/protect-pdf/");
    await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
    await page.setInputFiles("#file-in", {
      name: "secret.pdf",
      mimeType: "application/pdf",
      buffer: samplePdf(),
    });
    await page.locator('input[type="password"]').first().fill("hunter2demo");
    await page.locator(".actions button.primary").click();
    await expect(page.locator(".results")).toBeVisible({ timeout: 30_000 });
    const download = page.waitForEvent("download");
    await page.locator(".results button", { hasText: "Download" }).click();
    const protectedPdf = readFileSync(await (await download).path());

    await gotoEditor(page);
    await page.setInputFiles("#pdf-in", {
      name: "secret-protected.pdf",
      mimeType: "application/pdf",
      buffer: protectedPdf,
    });
    const err = page.locator(".error");
    await expect(err).toHaveCount(1, { timeout: 30_000 });
    await expect(err).toContainText("password-protected");
    await expect(err).toContainText("Unlock PDF");
    // None of the internals may reach the user.
    await expect(err).not.toContainText("PasswordException");
    await expect(err).not.toContainText("Communication");
    await expect(err).not.toContainText("PZ_ENCRYPTED");
    await expect(page.locator(".pz-page")).toHaveCount(0);
  });

  test("export downloads an edited PDF", async ({ page }) => {
    await openPdf(page);
    await page.locator("button", { hasText: "Export ↓" }).click();
    const download = page.waitForEvent("download");
    await page.locator("button", { hasText: "⬇ Download PDF" }).click();
    const dl = await download;
    expect(dl.suggestedFilename()).toBe("sample-edited.pdf");
  });
});
