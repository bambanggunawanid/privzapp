// tesseract-wasm shim for the OCR tools (ADR-0011).
//
// One OCRClient lives in a Web Worker (spawned by the bundled
// tesseract-wasm lib) and stays warm between runs; switching language
// just loads the other model into it. Everything loads same-origin from
// /ocr/ — the ESM lib resolves its worker, and the worker its wasm
// (SIMD or fallback), relative to their own URLs, which is why those
// files are served unhashed (see scripts/fetch-ocr.sh). Never a CDN.

const O = (window.pzOcr = window.pzOcr || { client: null, lang: null });

async function pzOcrInit(lang) {
  if (!O.client) {
    const { OCRClient } = await import("/ocr/lib.js");
    O.client = new OCRClient();
  }
  if (O.lang !== lang) {
    await O.client.loadModel(`/ocr/tessdata/${lang}.traineddata`);
    O.lang = lang;
  }
  return true;
}

// Recognize one image (a blob: URL) and return its text.
async function pzOcrImage(url, lang) {
  await pzOcrInit(lang);
  const blob = await (await fetch(url)).blob();
  let bitmap;
  try {
    bitmap = await createImageBitmap(blob);
  } catch {
    throw new Error("that file is not an image the browser can decode");
  }
  try {
    await O.client.loadImage(bitmap);
    return await O.client.getText();
  } finally {
    bitmap.close();
  }
}
