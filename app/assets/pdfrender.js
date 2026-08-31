// Headless PDF page rasterization for the PDF to Image tool (ADR-0009).
//
// Deliberately NOT part of editor.js: that module owns the editing
// workspace (overlay canvases, strokes, undo, a #pz-pages container it
// writes into) and is ~40 KB the tool page has no use for. This file only
// renders pages to offscreen canvases and hands the bytes back. It shares
// the same bundled PDF.js — never a CDN — so the offline and
// no-third-party promises hold.
//
// Rendering only. Nothing here mutates a PDF; the engine still owns every
// byte that gets written (ADR-0007 rule 1).

const R = (window.pzRender = window.pzRender || { lib: null });

async function pzRenderInit(pdfjsUrl, workerUrl) {
  if (!R.lib) {
    R.lib = await import(pdfjsUrl);
    R.lib.GlobalWorkerOptions.workerSrc = workerUrl;
  }
  return true;
}

function pzRenderBytes(b64) {
  const bin = atob(b64);
  const arr = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
  return arr;
}

// `pages` is a 1-based list; empty/omitted means every page. Returns
// [{page, data}] with `data` base64 — the Rust side decodes, names and
// packages them.
async function pzRenderDoc(params, scale, mime, quality, pages) {
  let doc;
  try {
    doc = await R.lib.getDocument(params).promise;
  } catch (e) {
    // Same marker as editor.js: an encrypted PDF is a user problem with
    // a clear fix, not an internal error to dump on them.
    if (e && (e.name === "PasswordException" || e.code === 1)) {
      throw new Error("PZ_ENCRYPTED");
    }
    throw e;
  }
  try {
    let wanted = Array.isArray(pages) && pages.length ? pages : null;
    if (!wanted) {
      wanted = [];
      for (let n = 1; n <= doc.numPages; n++) wanted.push(n);
    }
    const out = [];
    for (const n of wanted) {
      if (n < 1 || n > doc.numPages) {
        throw new Error(`page ${n} is out of range (1-${doc.numPages})`);
      }
      const page = await doc.getPage(n);
      const vp = page.getViewport({ scale: scale || 2 });
      const canvas = document.createElement("canvas");
      canvas.width = vp.width;
      canvas.height = vp.height;
      try {
        await page.render({
          canvasContext: canvas.getContext("2d"),
          viewport: vp,
        }).promise;
        const blob = await new Promise((res) =>
          canvas.toBlob(res, mime, quality),
        );
        if (!blob) throw new Error(`could not encode page ${n} as ${mime}`);
        const b64 = await new Promise((res, rej) => {
          const fr = new FileReader();
          fr.onload = () => res(fr.result.split(",", 2)[1]);
          fr.onerror = () => rej(new Error(`could not read page ${n}`));
          fr.readAsDataURL(blob);
        });
        out.push({ page: n, data: b64 });
      } finally {
        // Big documents at 4x add up fast — drop the backing store now
        // instead of waiting for GC.
        canvas.width = 0;
        canvas.height = 0;
      }
      page.cleanup();
    }
    return out;
  } finally {
    await doc.destroy();
  }
}

// Web path: `src` is a blob: URL, so megabytes never go through an eval
// string. pzRenderB64 is the fallback where blob URLs aren't available.
async function pzRenderUrl(src, scale, mime, quality, pages) {
  return pzRenderDoc({ url: src }, scale, mime, quality, pages);
}

async function pzRenderUrlB64(b64, scale, mime, quality, pages) {
  return pzRenderDoc(
    { data: pzRenderBytes(b64) },
    scale,
    mime,
    quality,
    pages,
  );
}
