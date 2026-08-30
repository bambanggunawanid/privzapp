// Post-deploy smoke: drives the REAL container (nginx + security headers)
// and fails if the CSP blocks anything the app needs — wasm boot, PDF.js
// module import, blob previews. The regular UI suite runs against a bare
// python server with no headers, so this is the only check that exercises
// deploy/security-headers.conf. Usage:
//   node csp-smoke.mjs http://127.0.0.1:8090
import { chromium } from "@playwright/test";
import { samplePng } from "./fixtures/sample-png.mjs";
import { samplePdf } from "./fixtures/sample-pdf.mjs";
import { sampleY4m } from "./fixtures/sample-y4m.mjs";

const base = process.argv[2] || "http://127.0.0.1:8090";
const browser = await chromium.launch();
const page = await browser.newPage();

const violations = [];
page.on("console", (msg) => {
  if (/Content.Security.Policy|Refused to/i.test(msg.text())) {
    violations.push(msg.text());
  }
});
page.on("pageerror", (err) => violations.push(`pageerror: ${err.message}`));

// Wasm must boot on a tool page and produce a blob preview.
await page.goto(`${base}/tool/compress-img/`);
await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
await page.setInputFiles("#file-in", {
  name: "smoke.png",
  mimeType: "image/png",
  buffer: samplePng(),
});
await page.waitForSelector(".preview-after", { timeout: 20_000 });

// The editor must import PDF.js (ES module) and render a page.
await page.goto(`${base}/tool/edit-pdf/`);
await page.waitForSelector("#pdf-in", { state: "attached", timeout: 45_000 });
await page.setInputFiles("#pdf-in", {
  name: "smoke.pdf",
  mimeType: "application/pdf",
  buffer: samplePdf(),
});
await page.waitForSelector(".pz-page canvas", { timeout: 30_000 });

// ffmpeg.wasm must spawn its worker and importScripts the core — the two
// moves most likely to trip script-src/worker-src.
await page.goto(`${base}/tool/video-to-gif/`);
await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
await page.setInputFiles("#file-in", {
  name: "smoke.y4m",
  mimeType: "video/x-yuv4mpeg",
  buffer: sampleY4m(),
});
await page.locator(".actions button.primary").click();
await page.waitForSelector(".results", { timeout: 90_000 });

// The OCR stack: ESM module import, its worker, and a worker-side fetch
// of the model — all must clear script-src/worker-src/connect-src.
await page.goto(`${base}/tool/image-to-text/`);
await page.waitForSelector("#file-in", { state: "attached", timeout: 45_000 });
const ocrPng = await page.evaluate(async () => {
  const canvas = document.createElement("canvas");
  canvas.width = 320; canvas.height = 80;
  const c = canvas.getContext("2d");
  c.fillStyle = "#fff"; c.fillRect(0, 0, 320, 80);
  c.fillStyle = "#000"; c.font = "bold 40px sans-serif";
  c.fillText("SMOKE 99", 20, 55);
  const blob = await new Promise((r) => canvas.toBlob(r, "image/png"));
  const fr = new FileReader();
  return new Promise((r) => { fr.onload = () => r(fr.result.split(",", 2)[1]); fr.readAsDataURL(blob); });
});
await page.setInputFiles("#file-in", {
  name: "smoke.png",
  mimeType: "image/png",
  buffer: Buffer.from(ocrPng, "base64"),
});
await page.locator(".actions button.primary").click();
await page.waitForSelector(".results", { timeout: 90_000 });

await browser.close();
if (violations.length) {
  console.error("CSP smoke FAILED:\n" + violations.join("\n"));
  process.exit(1);
}
console.log("CSP smoke passed: wasm boot, engine preview, PDF.js, ffmpeg.wasm and tesseract-wasm all fine under the deployed headers.");
