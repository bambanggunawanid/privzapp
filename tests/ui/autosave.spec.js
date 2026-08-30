// Editor autosave (ADR-0013): a refresh must not cost you the document.
// These drive the real bundle — IndexedDB, the AES seal/open round trip
// through Rust, and the restore offer — and check the shred on discard.
import { test, expect } from "@playwright/test";
import { samplePdf } from "./fixtures/sample-pdf.mjs";

test.setTimeout(90_000);

async function gotoEditor(page) {
  await page.goto("/tool/edit-pdf/");
  await page.waitForSelector("#pdf-in", { state: "attached", timeout: 45_000 });
}

async function openPdf(page, name = "contract.pdf") {
  await page.setInputFiles("#pdf-in", {
    name,
    mimeType: "application/pdf",
    buffer: samplePdf(),
  });
  await expect(page.locator(".pz-page")).toHaveCount(2, { timeout: 30_000 });
}

// What actually sits in storage, read from the page itself.
//
// This MUST create the object store the same way autosave.js does. An
// open() without onupgradeneeded that wins the race against the app
// creates an empty version-1 database, after which the app's own open
// never upgrades and autosave silently breaks — which is a bug in the
// probe, not the product, and it made these tests flaky until fixed.
function stored(page) {
  return page.evaluate(async () => {
    const key = localStorage.getItem("pz-ed-key");
    const rec = await new Promise((res) => {
      const req = indexedDB.open("pz-editor", 1);
      req.onupgradeneeded = () => {
        const db = req.result;
        if (!db.objectStoreNames.contains("doc")) db.createObjectStore("doc");
      };
      req.onsuccess = () => {
        const db = req.result;
        if (!db.objectStoreNames.contains("doc")) {
          db.close();
          return res(null);
        }
        const g = db.transaction("doc", "readonly").objectStore("doc").get("current");
        g.onsuccess = () => {
          db.close();
          res(g.result || null);
        };
        g.onerror = () => {
          db.close();
          res(null);
        };
      };
      req.onerror = () => res(null);
    });
    return {
      hasKey: !!key,
      name: rec ? rec.name : null,
      bytes: rec ? Array.from(rec.bytes.slice(0, 5)) : null,
      length: rec ? rec.bytes.length : 0,
    };
  });
}

test.describe("editor autosave", () => {
  test.beforeEach(async ({ page }) => {
    await gotoEditor(page);
    await page.evaluate(() => localStorage.removeItem("pz-ed-key"));
  });

  test("a refresh offers the document back instead of losing it", async ({ page }) => {
    await openPdf(page);
    // Give the autosave effect a moment to seal and store.
    await expect.poll(async () => (await stored(page)).name, { timeout: 20_000 }).toBe(
      "contract.pdf",
    );

    await page.reload();
    await page.waitForSelector("#pdf-in", { state: "attached", timeout: 45_000 });
    const offer = page.locator(".ed-restore");
    await expect(offer).toBeVisible({ timeout: 20_000 });
    await expect(offer).toContainText("contract.pdf");
    // Nothing is restored until asked — critical on a shared computer.
    await expect(page.locator(".pz-page")).toHaveCount(0);

    await offer.locator("button", { hasText: "Restore" }).click();
    await expect(page.locator(".pz-page")).toHaveCount(2, { timeout: 30_000 });
    await expect(offer).toHaveCount(0);
  });

  test("what is stored is encrypted, not the raw PDF", async ({ page }) => {
    await openPdf(page, "secret.pdf");
    await expect.poll(async () => (await stored(page)).length, { timeout: 20_000 }).toBeGreaterThan(
      0,
    );
    const rec = await stored(page);
    expect(rec.hasKey).toBe(true);
    // A real PDF starts "%PDF-"; sealed bytes must not.
    expect(Buffer.from(rec.bytes).toString("latin1")).not.toBe("%PDF-");
  });

  test("discard shreds the key and drops the document", async ({ page }) => {
    await openPdf(page);
    await expect.poll(async () => (await stored(page)).hasKey, { timeout: 20_000 }).toBe(true);

    await page.reload();
    await page.waitForSelector("#pdf-in", { state: "attached", timeout: 45_000 });
    await expect(page.locator(".ed-restore")).toBeVisible({ timeout: 20_000 });
    await page.locator(".ed-restore button", { hasText: "Discard" }).click();
    await expect(page.locator(".ed-restore")).toHaveCount(0);

    await expect.poll(async () => (await stored(page)).hasKey, { timeout: 10_000 }).toBe(false);
    await expect.poll(async () => (await stored(page)).name, { timeout: 10_000 }).toBe(null);

    // And it stays gone across another reload.
    await page.reload();
    await page.waitForSelector("#pdf-in", { state: "attached", timeout: 45_000 });
    await page.waitForTimeout(1_500);
    await expect(page.locator(".ed-restore")).toHaveCount(0);
  });

  test("a fresh visit with nothing saved shows no offer", async ({ page }) => {
    await page.evaluate(async () => {
      localStorage.removeItem("pz-ed-key");
      await new Promise((res) => {
        const r = indexedDB.deleteDatabase("pz-editor");
        r.onsuccess = r.onerror = r.onblocked = () => res();
      });
    });
    await page.reload();
    await page.waitForSelector("#pdf-in", { state: "attached", timeout: 45_000 });
    await page.waitForTimeout(1_500);
    await expect(page.locator(".ed-restore")).toHaveCount(0);
    await expect(page.locator(".dropzone")).toBeVisible();
  });
});
