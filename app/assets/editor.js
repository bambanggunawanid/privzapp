// PrivZapp PDF editor glue.
//
// Owns the PDF.js render (bundled locally — nothing is fetched from the
// network) and the per-page drawing overlays. The Rust side calls the pz*
// functions via dioxus eval and receives the final annotations from
// pzExport() in PDF coordinates, ready for the engine to bake in.
//
// Workspace extras (thumbnails, zoom, pan) live here too: they are pure
// presentation. Zoom re-renders pages at the new scale and rescales the
// stored overlay-pixel annotations by the same ratio, so pzExport()'s
// scale-relative math never changes.

window.pzEd = {
  lib: null,
  doc: null,
  pages: [], // 1-based: {wrap, canvas, overlay, ctx, scale, fitScale, pdfW, pdfH}
  tool: { mode: "pen", color: "#1130cc", size: 3 },
  strokes: {}, // page -> [{color,size,points:[[x,y],…]}] in overlay px
  images: {}, // page -> [{id,x,y,w,h}] in overlay px
  texts: {}, // page -> [{text,color,size,x,y}] in overlay px, y = 1st baseline
  bitmaps: {}, // image id -> ImageBitmap (preview only)
  staged: null, // image id waiting for placement
  stagedText: null, // {text,color,size} waiting for a tap
  history: [], // [{type:'stroke'|'image'|'text', page}]
  zoom: 1, // 1 = fit-width; survives re-opens so operations keep your view
  zooming: false,
};

async function pzInit(pdfjsUrl, workerUrl) {
  const E = window.pzEd;
  if (!E.lib) {
    E.lib = await import(pdfjsUrl);
    E.lib.GlobalWorkerOptions.workerSrc = workerUrl;
  }
  return true;
}

function pzB64(b64) {
  const bin = atob(b64);
  const arr = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
  return arr;
}

// `src` is a URL (usually a blob: URL from the Rust side — avoids pushing
// megabytes through eval strings). pzOpenB64 covers non-web builds.
async function pzOpen(src) {
  return pzOpenParams({ url: src });
}

async function pzOpenB64(b64) {
  return pzOpenParams({ data: pzB64(b64) });
}

async function pzOpenParams(params) {
  const E = window.pzEd;
  E.doc = await E.lib.getDocument(params).promise;
  E.pages = [];
  E.strokes = {};
  E.images = {};
  E.texts = {};
  E.bitmaps = {};
  E.history = [];
  E.staged = null;
  E.stagedText = null;
  const container = document.getElementById("pz-pages");
  container.innerHTML = "";
  const thumbs = document.getElementById("pz-thumbs");
  if (thumbs) thumbs.innerHTML = "";
  const maxW = Math.min((container.clientWidth || 848) - 48, 1100);
  for (let n = 1; n <= E.doc.numPages; n++) {
    const page = await E.doc.getPage(n);
    const base = page.getViewport({ scale: 1 });
    const fitScale = maxW / base.width;
    const scale = fitScale * E.zoom;
    const vp = page.getViewport({ scale });
    const wrap = document.createElement("div");
    wrap.className = "pz-page";
    wrap.style.width = vp.width + "px";
    wrap.style.height = vp.height + "px";
    const canvas = document.createElement("canvas");
    canvas.width = vp.width;
    canvas.height = vp.height;
    const overlay = document.createElement("canvas");
    overlay.width = vp.width;
    overlay.height = vp.height;
    overlay.className = "pz-overlay";
    if (E.tool.mode === "pan") overlay.style.pointerEvents = "none";
    wrap.appendChild(canvas);
    wrap.appendChild(overlay);
    container.appendChild(wrap);
    await page.render({ canvasContext: canvas.getContext("2d"), viewport: vp })
      .promise;
    E.pages[n] = {
      wrap,
      canvas,
      overlay,
      ctx: overlay.getContext("2d"),
      scale,
      fitScale,
      pdfW: base.width,
      pdfH: base.height,
    };
    pzHook(overlay, n);
    if (thumbs) await pzThumb(thumbs, page, base, n, wrap);
  }
  pzBindScroller(container);
  pzIndicate(1, E.doc.numPages);
  const z = document.getElementById("pz-zoomlvl");
  if (z) z.textContent = Math.round(E.zoom * 100) + "%";
  return E.doc.numPages;
}

async function pzThumb(thumbs, page, base, n, wrap) {
  const tscale = 96 / base.width;
  const tvp = page.getViewport({ scale: tscale });
  const tc = document.createElement("canvas");
  tc.width = tvp.width;
  tc.height = tvp.height;
  const item = document.createElement("div");
  item.className = n === 1 ? "pz-thumb active" : "pz-thumb";
  item.dataset.page = n;
  const num = document.createElement("span");
  num.textContent = n;
  item.appendChild(tc);
  item.appendChild(num);
  item.onclick = () => wrap.scrollIntoView({ behavior: "smooth", block: "start" });
  thumbs.appendChild(item);
  await page.render({ canvasContext: tc.getContext("2d"), viewport: tvp })
    .promise;
}

// Pan-drag, ctrl+wheel zoom and the current-page tracker, bound once on
// the workspace scroll container (the parent of #pz-pages).
function pzBindScroller(container) {
  const sc = container.parentElement;
  if (!sc || sc.dataset.pzBound) return;
  sc.dataset.pzBound = "1";

  let drag = null;
  sc.addEventListener("pointerdown", (ev) => {
    if (window.pzEd.tool.mode !== "pan") return;
    drag = { x: ev.clientX, y: ev.clientY, sx: sc.scrollLeft, sy: sc.scrollTop };
    sc.setPointerCapture(ev.pointerId);
  });
  sc.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    sc.scrollLeft = drag.sx - (ev.clientX - drag.x);
    sc.scrollTop = drag.sy - (ev.clientY - drag.y);
  });
  sc.addEventListener("pointerup", () => (drag = null));
  sc.addEventListener("pointercancel", () => (drag = null));

  sc.addEventListener(
    "wheel",
    (ev) => {
      if (!ev.ctrlKey) return;
      ev.preventDefault();
      pzZoom(ev.deltaY < 0 ? "in" : "out");
    },
    { passive: false },
  );

  let raf = 0;
  sc.addEventListener("scroll", () => {
    if (raf) return;
    raf = requestAnimationFrame(() => {
      raf = 0;
      pzTrackPage(sc);
    });
  });
}

function pzTrackPage(sc) {
  const E = window.pzEd;
  const mid = sc.getBoundingClientRect().top + sc.clientHeight * 0.4;
  let current = 1;
  for (let n = 1; n < E.pages.length + 1; n++) {
    const p = E.pages[n];
    if (p && p.wrap.getBoundingClientRect().top <= mid) current = n;
  }
  pzIndicate(current, E.doc ? E.doc.numPages : 0);
}

function pzIndicate(current, total) {
  const el = document.getElementById("pz-pageno");
  if (el) el.textContent = total ? current + " / " + total : "–";
  document.querySelectorAll(".pz-thumb").forEach((t) => {
    t.classList.toggle("active", Number(t.dataset.page) === current);
  });
}

// 'in' | 'out' | 'fit'. Re-renders every page at the new scale and rescales
// stored annotations so drawings stay glued to the page content.
async function pzZoom(action) {
  const E = window.pzEd;
  if (E.zooming || !E.doc) return Math.round(E.zoom * 100);
  let z = E.zoom;
  if (action === "in") z = Math.min(z * 1.25, 4);
  else if (action === "out") z = Math.max(z / 1.25, 0.3);
  else z = 1;
  if (Math.abs(z - E.zoom) < 1e-4) return Math.round(z * 100);
  E.zooming = true;
  E.zoom = z;
  try {
    for (let n = 1; n < E.pages.length + 1; n++) {
      const p = E.pages[n];
      if (!p) continue;
      const page = await E.doc.getPage(n);
      const newScale = p.fitScale * z;
      const ratio = newScale / p.scale;
      const vp = page.getViewport({ scale: newScale });
      p.wrap.style.width = vp.width + "px";
      p.wrap.style.height = vp.height + "px";
      p.canvas.width = vp.width;
      p.canvas.height = vp.height;
      p.overlay.width = vp.width;
      p.overlay.height = vp.height;
      await page.render({
        canvasContext: p.canvas.getContext("2d"),
        viewport: vp,
      }).promise;
      for (const s of window.pzEd.strokes[n] || []) {
        s.size *= ratio;
        s.points = s.points.map(([x, y]) => [x * ratio, y * ratio]);
      }
      for (const im of window.pzEd.images[n] || []) {
        im.x *= ratio;
        im.y *= ratio;
        im.w *= ratio;
        im.h *= ratio;
      }
      for (const t of window.pzEd.texts[n] || []) {
        t.x *= ratio;
        t.y *= ratio;
        t.size *= ratio;
      }
      p.scale = newScale;
      p.ctx = p.overlay.getContext("2d");
      pzRedraw(n);
    }
  } finally {
    E.zooming = false;
  }
  const el = document.getElementById("pz-zoomlvl");
  if (el) el.textContent = Math.round(z * 100) + "%";
  return Math.round(z * 100);
}

function pzPos(overlay, ev) {
  const r = overlay.getBoundingClientRect();
  return [ev.clientX - r.left, ev.clientY - r.top];
}

function pzHook(overlay, n) {
  const E = window.pzEd;
  let stroke = null;
  let rectStart = null;

  overlay.addEventListener("pointerdown", (ev) => {
    ev.preventDefault();
    overlay.setPointerCapture(ev.pointerId);
    const [x, y] = pzPos(overlay, ev);
    if (E.tool.mode === "image" && E.staged) {
      rectStart = [x, y];
    } else if (E.tool.mode === "text" && E.stagedText) {
      (E.texts[n] = E.texts[n] || []).push({
        ...E.stagedText,
        x,
        y,
      });
      E.history.push({ type: "text", page: n });
      E.stagedText = null;
      E.tool.mode = "pen";
      pzRedraw(n);
    } else {
      stroke = { color: E.tool.color, size: E.tool.size, points: [[x, y]] };
    }
  });

  overlay.addEventListener("pointermove", (ev) => {
    const [x, y] = pzPos(overlay, ev);
    if (stroke) {
      const pts = stroke.points;
      const [lx, ly] = pts[pts.length - 1];
      if ((x - lx) ** 2 + (y - ly) ** 2 < 4) return; // thin out points
      pts.push([x, y]);
      const ctx = E.pages[n].ctx;
      ctx.strokeStyle = stroke.color;
      ctx.lineWidth = stroke.size;
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.moveTo(lx, ly);
      ctx.lineTo(x, y);
      ctx.stroke();
    } else if (rectStart) {
      pzRedraw(n);
      const ctx = E.pages[n].ctx;
      ctx.save();
      ctx.strokeStyle = "#6c8cff";
      ctx.setLineDash([6, 4]);
      ctx.strokeRect(
        rectStart[0],
        rectStart[1],
        x - rectStart[0],
        y - rectStart[1],
      );
      ctx.restore();
    }
  });

  const finish = (ev) => {
    const [x, y] = pzPos(overlay, ev);
    if (stroke) {
      (E.strokes[n] = E.strokes[n] || []).push(stroke);
      E.history.push({ type: "stroke", page: n });
      pzRedraw(n);
      stroke = null;
    } else if (rectStart) {
      let [x0, y0] = rectStart;
      let w = x - x0;
      let h = y - y0;
      if (w < 0) {
        x0 += w;
        w = -w;
      }
      if (h < 0) {
        y0 += h;
        h = -h;
      }
      if (w > 8 && h > 8) {
        (E.images[n] = E.images[n] || []).push({
          id: E.staged,
          x: x0,
          y: y0,
          w,
          h,
        });
        E.history.push({ type: "image", page: n });
        E.staged = null;
        E.tool.mode = "pen";
      }
      rectStart = null;
      pzRedraw(n);
    }
  };
  overlay.addEventListener("pointerup", finish);
  overlay.addEventListener("pointercancel", () => {
    stroke = null;
    rectStart = null;
  });
}

function pzRedraw(n) {
  const E = window.pzEd;
  const p = E.pages[n];
  p.ctx.clearRect(0, 0, p.overlay.width, p.overlay.height);
  for (const im of E.images[n] || []) {
    const bmp = E.bitmaps[im.id];
    if (bmp) p.ctx.drawImage(bmp, im.x, im.y, im.w, im.h);
  }
  for (const t of E.texts[n] || []) {
    p.ctx.fillStyle = t.color;
    p.ctx.font = t.size + "px Helvetica, Arial, sans-serif";
    p.ctx.textBaseline = "alphabetic";
    t.text.split("\n").forEach((line, i) => {
      p.ctx.fillText(line, t.x, t.y + i * t.size * 1.25);
    });
  }
  for (const s of E.strokes[n] || []) {
    p.ctx.strokeStyle = s.color;
    p.ctx.lineWidth = s.size;
    p.ctx.lineCap = "round";
    p.ctx.lineJoin = "round";
    p.ctx.beginPath();
    s.points.forEach(([x, y], i) =>
      i === 0 ? p.ctx.moveTo(x, y) : p.ctx.lineTo(x, y),
    );
    if (s.points.length === 1) p.ctx.lineTo(s.points[0][0], s.points[0][1]);
    p.ctx.stroke();
  }
}

function pzSetTool(mode, color, size) {
  const E = window.pzEd;
  E.tool = { mode, color, size };
  if (mode !== "image") E.staged = null;
  const pan = mode === "pan";
  for (const p of E.pages) {
    if (p) p.overlay.style.pointerEvents = pan ? "none" : "auto";
  }
  const container = document.getElementById("pz-pages");
  if (container && container.parentElement) {
    container.parentElement.classList.toggle("pz-panning", pan);
  }
  return true;
}

async function pzStageImage(id, url) {
  const blob = await (await fetch(url)).blob();
  return pzStageBlob(id, blob);
}

async function pzStageImageB64(id, b64) {
  return pzStageBlob(id, new Blob([pzB64(b64)]));
}

async function pzStageBlob(id, blob) {
  const E = window.pzEd;
  E.bitmaps[id] = await createImageBitmap(blob);
  E.staged = id;
  pzSetTool("image", E.tool.color, E.tool.size);
  E.staged = id; // pzSetTool clears staged for other modes; keep it
  return true;
}

function pzStageText(text, color, size) {
  const E = window.pzEd;
  E.stagedText = { text, color, size };
  pzSetTool("text", color, E.tool.size);
  return true;
}

function pzUndo() {
  const E = window.pzEd;
  const last = E.history.pop();
  if (!last) return false;
  const list =
    last.type === "stroke"
      ? E.strokes
      : last.type === "text"
        ? E.texts
        : E.images;
  (list[last.page] || []).pop();
  pzRedraw(last.page);
  return true;
}

// Annotations in PDF coordinates (points, origin bottom-left).
function pzExport() {
  const E = window.pzEd;
  const out = [];
  for (let n = 1; n < E.pages.length + 1; n++) {
    const p = E.pages[n];
    if (!p) continue;
    const strokes = (E.strokes[n] || []).map((s) => ({
      color: s.color,
      width: s.size / p.scale,
      points: s.points.map(([x, y]) => [x / p.scale, p.pdfH - y / p.scale]),
    }));
    const images = (E.images[n] || []).map((im) => ({
      id: im.id,
      rect: [
        im.x / p.scale,
        p.pdfH - (im.y + im.h) / p.scale,
        im.w / p.scale,
        im.h / p.scale,
      ],
    }));
    const texts = (E.texts[n] || []).map((t) => ({
      text: t.text,
      color: t.color,
      size: t.size / p.scale,
      x: t.x / p.scale,
      y: p.pdfH - t.y / p.scale,
    }));
    if (strokes.length || images.length || texts.length)
      out.push({ page: n, strokes, images, texts });
  }
  return out;
}
