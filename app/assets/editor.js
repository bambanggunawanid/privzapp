// PrivZapp PDF editor glue.
//
// Owns the PDF.js render (bundled locally — nothing is fetched from the
// network), the per-page drawing overlays and the live text objects. The
// Rust side calls the pz* functions via dioxus eval and receives the final
// annotations from pzExport() in PDF coordinates, ready for the engine to
// bake in.
//
// Layering inside each .pz-page wrap (bottom → top), mirroring the
// engine's paint order (rects → images → texts → ink):
//   1. base canvas    — the PDF.js render
//   2. .pz-textlayer  — transparent selectable text (detection for retype)
//   3. .pz-under      — white-out rects + staged image previews
//   4. .pz-text divs  — live, editable text boxes (contenteditable)
//   5. .pz-overlay    — ink strokes; the pointer target for drawing tools
//
// Tools: cursor (default; select/move/edit text, click detected text to
// retype), pan, pen, highlight (translucent multiply), text, image,
// redact (drag a box; on bake the Rust engine REMOVES the text under it
// from the content stream — true redaction, not a cover-up).

window.pzEd = {
  lib: null,
  doc: null,
  pages: [], // 1-based: {wrap, canvas, under, uctx, overlay, ctx, scale, fitScale, pdfW, pdfH}
  tool: { mode: "cursor", color: "#1130cc", size: 3, opacity: 1 },
  strokes: {}, // page -> [{color,size,opacity,points:[[x,y],…]}] overlay px
  images: {}, // page -> [{el,id,x,y,w,h,opacity}] live DOM objects, overlay px
  texts: {}, // page -> [{el,x,y,size,color,bold}] top-left overlay px
  rects: {}, // page -> [{x,y,w,h,spanEl}] white-outs, overlay px
  redacts: {}, // page -> [{el,x,y,w,h}] pending redaction boxes, overlay px
  history: [], // [{type:'stroke'|'image'|'image-del'|'text'|'retype'|'redact', page, obj?}]
  redo: [], // popped history entries with payloads
  zoom: 1, // 1 = fit-width; survives re-opens so operations keep your view
  zooming: false,
  dragThumb: null,
  current: 1, // page nearest the viewport center (image placement target)
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
  E.rects = {};
  E.redacts = {};
  E.history = [];
  E.redo = [];
  E.current = 1;
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
    wrap.style.setProperty("--scale-factor", scale);
    const canvas = document.createElement("canvas");
    canvas.width = vp.width;
    canvas.height = vp.height;
    const tl = document.createElement("div");
    tl.className = "pz-textlayer textLayer";
    const under = document.createElement("canvas");
    under.width = vp.width;
    under.height = vp.height;
    under.className = "pz-under";
    const overlay = document.createElement("canvas");
    overlay.width = vp.width;
    overlay.height = vp.height;
    overlay.className = "pz-overlay";
    wrap.appendChild(canvas);
    wrap.appendChild(tl);
    wrap.appendChild(under);
    wrap.appendChild(overlay);
    container.appendChild(wrap);
    await page.render({ canvasContext: canvas.getContext("2d"), viewport: vp })
      .promise;
    E.pages[n] = {
      wrap,
      canvas,
      under,
      uctx: under.getContext("2d"),
      overlay,
      ctx: overlay.getContext("2d"),
      scale,
      fitScale,
      pdfW: base.width,
      pdfH: base.height,
    };
    // Selectable/detectable text under everything (retype source).
    try {
      const textLayer = new E.lib.TextLayer({
        textContentSource: page.streamTextContent(),
        container: tl,
        viewport: vp,
      });
      await textLayer.render();
    } catch (e) {
      // No text layer (scanned page / older API) — retype simply unavailable.
    }
    pzHook(overlay, n);
    pzHookWrap(wrap, n);
    if (thumbs) await pzThumb(thumbs, page, base, n, wrap);
  }
  pzBindScroller(container);
  pzApplyMode();
  pzIndicate(1, E.doc.numPages);
  const z = document.getElementById("pz-zoomlvl");
  if (z) z.textContent = Math.round(E.zoom * 100) + "%";
  const grid = document.getElementById("pz-pages");
  if (grid && E.pages[1]) {
    grid.style.setProperty("--pz-grid", E.pages[1].scale * 24 + "px");
  }
  pzDrawRulers();
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
  item.draggable = true;
  const num = document.createElement("span");
  num.textContent = n;
  item.appendChild(tc);
  item.appendChild(num);
  item.onclick = () =>
    wrap.scrollIntoView({ behavior: "smooth", block: "start" });
  pzThumbDnD(thumbs, item);
  thumbs.appendChild(item);
  await page.render({ canvasContext: tc.getContext("2d"), viewport: tvp })
    .promise;
}

// Drag & drop page reordering in the thumbnail rail. On drop the new
// order is sent to Rust (window.pzNotify), which runs the reorder
// operation and re-renders.
function pzThumbDnD(thumbs, item) {
  const E = window.pzEd;
  item.addEventListener("dragstart", (e) => {
    E.dragThumb = item;
    item.classList.add("dragging");
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", item.dataset.page);
  });
  item.addEventListener("dragend", () => {
    item.classList.remove("dragging");
    E.dragThumb = null;
  });
  if (thumbs.dataset.pzDnd) return;
  thumbs.dataset.pzDnd = "1";
  const orderBefore = () =>
    [...thumbs.querySelectorAll(".pz-thumb")].map((c) => c.dataset.page).join(",");
  let startOrder = "";
  thumbs.addEventListener("dragenter", () => {
    if (!startOrder) startOrder = orderBefore();
  });
  thumbs.addEventListener("dragover", (e) => {
    e.preventDefault();
    const drag = E.dragThumb;
    if (!drag) return;
    const others = [...thumbs.querySelectorAll(".pz-thumb:not(.dragging)")];
    const next = others.find(
      (c) => e.clientY < c.getBoundingClientRect().top + c.offsetHeight / 2,
    );
    if (next) thumbs.insertBefore(drag, next);
    else thumbs.appendChild(drag);
  });
  thumbs.addEventListener("drop", (e) => {
    e.preventDefault();
    const order = orderBefore();
    if (startOrder && order !== startOrder && window.pzNotify) {
      window.pzNotify("reorder:" + order);
    }
    startOrder = "";
  });
}

// Pan-drag, ctrl+wheel zoom, ruler redraw and the current-page tracker,
// bound once on the workspace scroll container (parent of #pz-pages).
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
      pzDrawRulers();
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
  E.current = current;
  pzIndicate(current, E.doc ? E.doc.numPages : 0);
}

function pzIndicate(current, total) {
  const el = document.getElementById("pz-pageno");
  if (el) el.textContent = total ? current + " / " + total : "–";
  document.querySelectorAll(".pz-thumb").forEach((t) => {
    t.classList.toggle("active", Number(t.dataset.page) === current);
  });
}

// ---- views: ruler + grid ----

function pzView(kind, on) {
  const container = document.getElementById("pz-pages");
  if (!container) return false;
  const sc = container.parentElement;
  const wrap = sc && sc.parentElement;
  if (kind === "grid") container.classList.toggle("pz-grid-on", on);
  if (kind === "ruler" && wrap) {
    wrap.classList.toggle("ed-rulers", on);
    pzDrawRulers();
  }
  return true;
}

// Rulers show PDF points, anchored to the first page's top-left corner.
function pzDrawRulers() {
  const E = window.pzEd;
  const h = document.getElementById("pz-ruler-h");
  const v = document.getElementById("pz-ruler-v");
  const container = document.getElementById("pz-pages");
  if (!h || !v || !container) return;
  const sc = container.parentElement;
  const wrap = sc.parentElement;
  if (!wrap.classList.contains("ed-rulers")) return;
  const p = E.pages[1];
  const style = getComputedStyle(document.documentElement);
  const ink = "rgba(154,164,184,0.9)";
  for (const [canvas, horiz] of [
    [h, true],
    [v, false],
  ]) {
    const cw = canvas.clientWidth || 1;
    const ch = canvas.clientHeight || 1;
    canvas.width = cw;
    canvas.height = ch;
    const ctx = canvas.getContext("2d");
    ctx.clearRect(0, 0, cw, ch);
    if (!p) continue;
    const scRect = sc.getBoundingClientRect();
    const pRect = p.wrap.getBoundingClientRect();
    const origin = horiz ? pRect.left - scRect.left : pRect.top - scRect.top;
    const span = horiz ? cw : ch;
    const scale = p.scale; // px per PDF point
    const step = scale >= 1.5 ? 10 : scale >= 0.7 ? 25 : 50;
    ctx.strokeStyle = ink;
    ctx.fillStyle = ink;
    ctx.font = "9px ui-sans-serif, system-ui, sans-serif";
    ctx.beginPath();
    const first = Math.floor(-origin / (step * scale)) * step;
    for (let pt = first; ; pt += step) {
      const at = origin + pt * scale;
      if (at > span) break;
      if (at < 0) continue;
      const major = pt % (step * 5) === 0;
      const len = major ? 10 : 5;
      if (horiz) {
        ctx.moveTo(at, ch);
        ctx.lineTo(at, ch - len);
        if (major) ctx.fillText(String(pt), at + 2, 9);
      } else {
        ctx.moveTo(cw, at);
        ctx.lineTo(cw - len, at);
        if (major) {
          ctx.save();
          ctx.translate(9, at + 2);
          ctx.rotate(-Math.PI / 2);
          ctx.fillText(String(pt), -ctx.measureText(String(pt)).width, 0);
          ctx.restore();
        }
      }
    }
    ctx.stroke();
  }
  void style;
}

// ---- zoom ----

// 'in' | 'out' | 'fit'. Re-renders every page at the new scale and
// rescales stored annotations so drawings stay glued to the content.
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
      p.wrap.style.setProperty("--scale-factor", newScale);
      p.canvas.width = vp.width;
      p.canvas.height = vp.height;
      p.under.width = vp.width;
      p.under.height = vp.height;
      p.overlay.width = vp.width;
      p.overlay.height = vp.height;
      await page.render({
        canvasContext: p.canvas.getContext("2d"),
        viewport: vp,
      }).promise;
      for (const s of E.strokes[n] || []) {
        s.size *= ratio;
        s.points = s.points.map(([x, y]) => [x * ratio, y * ratio]);
      }
      for (const im of E.images[n] || []) {
        im.x *= ratio;
        im.y *= ratio;
        im.w *= ratio;
        im.h *= ratio;
        im.el.style.left = im.x + "px";
        im.el.style.top = im.y + "px";
        im.el.style.width = im.w + "px";
        im.el.style.height = im.h + "px";
      }
      for (const r of E.rects[n] || []) {
        r.x *= ratio;
        r.y *= ratio;
        r.w *= ratio;
        r.h *= ratio;
      }
      for (const rd of E.redacts[n] || []) {
        rd.x *= ratio;
        rd.y *= ratio;
        rd.w *= ratio;
        rd.h *= ratio;
        rd.el.style.left = rd.x + "px";
        rd.el.style.top = rd.y + "px";
        rd.el.style.width = rd.w + "px";
        rd.el.style.height = rd.h + "px";
      }
      for (const t of E.texts[n] || []) {
        t.x *= ratio;
        t.y *= ratio;
        t.size *= ratio;
        t.el.style.left = t.x + "px";
        t.el.style.top = t.y + "px";
        t.el.style.fontSize = t.size + "px";
      }
      p.scale = newScale;
      p.uctx = p.under.getContext("2d");
      p.ctx = p.overlay.getContext("2d");
      pzRedraw(n);
    }
  } finally {
    E.zooming = false;
  }
  const el = document.getElementById("pz-zoomlvl");
  if (el) el.textContent = Math.round(z * 100) + "%";
  const grid = document.getElementById("pz-pages");
  if (grid && E.pages[1]) {
    grid.style.setProperty("--pz-grid", E.pages[1].scale * 24 + "px");
  }
  pzDrawRulers();
  return Math.round(z * 100);
}

// ---- pointer handling ----

function pzPos(el, ev) {
  const r = el.getBoundingClientRect();
  return [ev.clientX - r.left, ev.clientY - r.top];
}

function pzPushHistory(entry) {
  const E = window.pzEd;
  E.history.push(entry);
  E.redo = [];
}

// Drawing tools (pen/highlight/redact) — pointer events on the top overlay.
function pzHook(overlay, n) {
  const E = window.pzEd;
  let stroke = null;
  let redact = null; // {x0, y0} drag anchor while a redact box is drawn

  overlay.addEventListener("pointerdown", (ev) => {
    ev.preventDefault();
    overlay.setPointerCapture(ev.pointerId);
    const [x, y] = pzPos(overlay, ev);
    if (E.tool.mode === "pen" || E.tool.mode === "highlight") {
      stroke = {
        color: E.tool.color,
        size: E.tool.size,
        opacity: E.tool.opacity,
        points: [[x, y]],
      };
    } else if (E.tool.mode === "redact") {
      redact = { x0: x, y0: y };
    }
  });

  overlay.addEventListener("pointermove", (ev) => {
    if (redact) {
      // Live preview of the box being dragged.
      const [x, y] = pzPos(overlay, ev);
      pzRedraw(n);
      const ctx = E.pages[n].ctx;
      ctx.save();
      ctx.fillStyle = "rgba(0,0,0,0.55)";
      ctx.strokeStyle = "#e11";
      ctx.setLineDash([4, 3]);
      const rx = Math.min(redact.x0, x);
      const ry = Math.min(redact.y0, y);
      ctx.fillRect(rx, ry, Math.abs(x - redact.x0), Math.abs(y - redact.y0));
      ctx.strokeRect(rx, ry, Math.abs(x - redact.x0), Math.abs(y - redact.y0));
      ctx.restore();
      return;
    }
    if (!stroke) return;
    const [x, y] = pzPos(overlay, ev);
    const pts = stroke.points;
    const [lx, ly] = pts[pts.length - 1];
    if ((x - lx) ** 2 + (y - ly) ** 2 < 4) return; // thin out points
    pts.push([x, y]);
    const ctx = E.pages[n].ctx;
    ctx.save();
    ctx.globalAlpha = stroke.opacity;
    ctx.strokeStyle = stroke.color;
    ctx.lineWidth = stroke.size;
    ctx.lineCap = "round";
    ctx.beginPath();
    ctx.moveTo(lx, ly);
    ctx.lineTo(x, y);
    ctx.stroke();
    ctx.restore();
  });

  const finish = (ev) => {
    if (redact) {
      const [x, y] = pzPos(overlay, ev);
      const rx = Math.min(redact.x0, x);
      const ry = Math.min(redact.y0, y);
      const w = Math.abs(x - redact.x0);
      const h = Math.abs(y - redact.y0);
      redact = null;
      pzRedraw(n);
      if (w > 4 && h > 4) {
        const rec = pzMakeRedact(n, rx, ry, w, h);
        pzPushHistory({ type: "redact", page: n, obj: rec });
        rec.el.focus();
      }
      return;
    }
    if (!stroke) return;
    (E.strokes[n] = E.strokes[n] || []).push(stroke);
    pzPushHistory({ type: "stroke", page: n });
    pzRedraw(n);
    stroke = null;
  };
  overlay.addEventListener("pointerup", finish);
  overlay.addEventListener("pointercancel", () => {
    stroke = null;
    redact = null;
    pzRedraw(n);
  });
}

// ---- redaction boxes ----
// A pending box is a live object (drag to move, ✕ / Delete to remove).
// The permanent part happens in Rust at bake time: glyphs under the box
// are stripped from the content stream, then the area is painted black.

function pzMakeRedact(n, x, y, w, h) {
  const E = window.pzEd;
  const p = E.pages[n];
  const el = document.createElement("div");
  el.className = "pz-redact";
  el.tabIndex = 0;
  el.title = "Redaction — text under this box is removed on export";
  el.style.left = x + "px";
  el.style.top = y + "px";
  el.style.width = w + "px";
  el.style.height = h + "px";
  const del = document.createElement("button");
  del.className = "pz-obj-del";
  del.textContent = "✕";
  del.title = "Remove redaction";
  el.appendChild(del);
  const rec = { el, x, y, w, h };
  del.addEventListener("click", (ev) => {
    ev.stopPropagation();
    pzDeleteRedact(n, rec, true);
  });
  el.addEventListener("keydown", (ev) => {
    if (ev.key === "Delete" || ev.key === "Backspace") {
      ev.preventDefault();
      pzDeleteRedact(n, rec, true);
    }
  });
  el.addEventListener("pointerdown", (ev) => pzRedactPointer(ev, n, rec));
  p.wrap.insertBefore(el, p.overlay);
  (E.redacts[n] = E.redacts[n] || []).push(rec);
  return rec;
}

function pzDeleteRedact(n, rec, history) {
  const E = window.pzEd;
  rec.el.remove();
  const arr = E.redacts[n] || [];
  const i = arr.indexOf(rec);
  if (i >= 0) arr.splice(i, 1);
  if (history) pzPushHistory({ type: "redact-del", page: n, obj: rec });
}

function pzRedactPointer(ev, n, rec) {
  if (ev.target.tagName === "BUTTON") return;
  ev.preventDefault();
  rec.el.focus();
  rec.el.setPointerCapture(ev.pointerId);
  const sx = ev.clientX;
  const sy = ev.clientY;
  const ox = rec.x;
  const oy = rec.y;
  const mv = (e) => {
    rec.x = ox + e.clientX - sx;
    rec.y = oy + e.clientY - sy;
    rec.el.style.left = rec.x + "px";
    rec.el.style.top = rec.y + "px";
  };
  const up = () => {
    rec.el.removeEventListener("pointermove", mv);
    rec.el.removeEventListener("pointerup", up);
  };
  rec.el.addEventListener("pointermove", mv);
  rec.el.addEventListener("pointerup", up);
}

// Cursor/text tools — clicks that reach the page wrap (the overlay is
// pointer-transparent in those modes).
function pzHookWrap(wrap, n) {
  const E = window.pzEd;
  wrap.addEventListener("click", (ev) => {
    const mode = E.tool.mode;
    if (ev.target.closest(".pz-text")) return; // handled by the box itself
    const span = ev.target.closest(".pz-textlayer span");
    if (span && span.textContent.trim() && (mode === "cursor" || mode === "text")) {
      pzRetype(n, span);
      return;
    }
    if (mode === "text") {
      const [x, y] = pzPos(E.pages[n].overlay, ev);
      const size = E.tool.size;
      const rec = pzMakeText(n, x, y - size * 0.5, {
        text: "",
        size,
        color: E.tool.color,
      });
      pzPushHistory({ type: "text", page: n, obj: rec });
      rec.el.focus();
    }
  });
}

// ---- live text objects ----

function pzMakeText(n, x, y, opts) {
  const E = window.pzEd;
  const p = E.pages[n];
  const el = document.createElement("div");
  el.className = "pz-text";
  el.contentEditable = "true";
  el.spellcheck = false;
  el.style.left = x + "px";
  el.style.top = y + "px";
  el.style.fontSize = opts.size + "px";
  el.style.color = opts.color;
  if (opts.bold) el.style.fontWeight = "bold";
  el.textContent = opts.text || "";
  const rec = { el, x, y, size: opts.size, color: opts.color, bold: !!opts.bold };
  el.addEventListener("blur", () => {
    if (!el.innerText.trim()) pzRemoveText(n, rec, true);
  });
  el.addEventListener("pointerdown", (ev) => pzTextPointer(ev, n, rec));
  p.wrap.insertBefore(el, p.overlay);
  (E.texts[n] = E.texts[n] || []).push(rec);
  return rec;
}

function pzRemoveText(n, rec, alsoHistory) {
  const E = window.pzEd;
  rec.el.remove();
  const arr = E.texts[n] || [];
  const i = arr.indexOf(rec);
  if (i >= 0) arr.splice(i, 1);
  if (alsoHistory) {
    E.history = E.history.filter((h) => h.obj !== rec && (!h.obj || h.obj.box !== rec));
  }
}

// Cursor tool: drag moves the box; a plain click falls through to focus.
function pzTextPointer(ev, n, rec) {
  const E = window.pzEd;
  if (E.tool.mode !== "cursor" && E.tool.mode !== "text") return;
  if (document.activeElement === rec.el) return; // editing — let the caret work
  const startX = ev.clientX;
  const startY = ev.clientY;
  const origX = rec.x;
  const origY = rec.y;
  let moved = false;
  const onMove = (mv) => {
    const dx = mv.clientX - startX;
    const dy = mv.clientY - startY;
    if (!moved && dx * dx + dy * dy < 16) return;
    moved = true;
    rec.x = origX + dx;
    rec.y = origY + dy;
    rec.el.style.left = rec.x + "px";
    rec.el.style.top = rec.y + "px";
  };
  const onUp = () => {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    if (moved) rec.el.blur();
  };
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
}

// The source text's color, sampled from the rendered pixels under the
// span: the most frequent color bucket is the background; the answer is
// the average of pixels far from it. Editing must not restyle the text.
function pzSampleTextColor(p, x, y, w, h) {
  try {
    const sx = Math.max(0, Math.round(x));
    const sy = Math.max(0, Math.round(y));
    const sw = Math.min(p.canvas.width - sx, Math.max(1, Math.round(w)));
    const sh = Math.min(p.canvas.height - sy, Math.max(1, Math.round(h)));
    if (sw < 2 || sh < 2) return "#000000";
    const data = p.canvas.getContext("2d").getImageData(sx, sy, sw, sh).data;
    const buckets = new Map();
    for (let i = 0; i < data.length; i += 4) {
      const key = ((data[i] >> 4) << 8) | ((data[i + 1] >> 4) << 4) | (data[i + 2] >> 4);
      buckets.set(key, (buckets.get(key) || 0) + 1);
    }
    let bg = 0xfff;
    let max = -1;
    for (const [k, c] of buckets) {
      if (c > max) {
        max = c;
        bg = k;
      }
    }
    const bgc = [((bg >> 8) & 15) * 17, ((bg >> 4) & 15) * 17, (bg & 15) * 17];
    let r = 0;
    let g = 0;
    let b = 0;
    let count = 0;
    for (let i = 0; i < data.length; i += 4) {
      const d =
        Math.abs(data[i] - bgc[0]) +
        Math.abs(data[i + 1] - bgc[1]) +
        Math.abs(data[i + 2] - bgc[2]);
      if (d > 120) {
        r += data[i];
        g += data[i + 1];
        b += data[i + 2];
        count++;
      }
    }
    if (!count) return "#000000";
    const hex = (v) => Math.round(v / count).toString(16).padStart(2, "0");
    return "#" + hex(r) + hex(g) + hex(b);
  } catch (e) {
    return "#000000";
  }
}

// Bold if the span's real advance width is closer to bold metrics than
// regular ones.
function pzGuessBold(text, fs, targetW) {
  if (!text.trim() || targetW < 4) return false;
  const c = document.createElement("canvas").getContext("2d");
  c.font = fs + "px Helvetica, Arial, sans-serif";
  const wn = c.measureText(text).width;
  c.font = "bold " + fs + "px Helvetica, Arial, sans-serif";
  const wb = c.measureText(text).width;
  return Math.abs(wb - targetW) < Math.abs(wn - targetW);
}

// Cover & retype: white-out the detected text and drop an editable box
// with the same content on top, inheriting the source style (sampled
// color, guessed weight, span font size) — editing changes content, not
// style. Works best on white backgrounds.
function pzRetype(n, span) {
  const E = window.pzEd;
  const p = E.pages[n];
  const wr = p.wrap.getBoundingClientRect();
  const sr = span.getBoundingClientRect();
  const x = sr.left - wr.left;
  const y = sr.top - wr.top;
  const fs = parseFloat(getComputedStyle(span).fontSize) || sr.height * 0.9;
  const color = pzSampleTextColor(p, x, y, sr.width, sr.height);
  const bold = pzGuessBold(span.textContent, fs, sr.width);
  const rect = {
    x: x - 2,
    y: y - 2,
    w: sr.width + 4,
    h: sr.height + 4,
    spanEl: span,
  };
  (E.rects[n] = E.rects[n] || []).push(rect);
  span.style.visibility = "hidden";
  pzRedraw(n);
  const rec = pzMakeText(n, x, y, {
    text: span.textContent,
    size: fs,
    color,
    bold,
  });
  pzPushHistory({ type: "retype", page: n, obj: { box: rec, rect } });
  rec.el.focus();
  return true;
}

// ---- rendering ----

function pzRedraw(n) {
  const E = window.pzEd;
  const p = E.pages[n];
  // Under layer: white-outs (images are live DOM objects now).
  p.uctx.clearRect(0, 0, p.under.width, p.under.height);
  for (const r of E.rects[n] || []) {
    p.uctx.fillStyle = "#ffffff";
    p.uctx.fillRect(r.x, r.y, r.w, r.h);
  }
  // Ink layer.
  p.ctx.clearRect(0, 0, p.overlay.width, p.overlay.height);
  for (const s of E.strokes[n] || []) {
    p.ctx.save();
    p.ctx.globalAlpha = s.opacity == null ? 1 : s.opacity;
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
    p.ctx.restore();
  }
}

// ---- tools ----

function pzApplyMode() {
  const E = window.pzEd;
  const mode = E.tool.mode;
  const drawing = mode === "pen" || mode === "highlight" || mode === "redact";
  for (const p of E.pages) {
    if (p) p.overlay.style.pointerEvents = drawing ? "auto" : "none";
  }
  const container = document.getElementById("pz-pages");
  if (container && container.parentElement) {
    const sc = container.parentElement;
    sc.dataset.pzMode = mode;
    sc.classList.toggle("pz-panning", mode === "pan");
  }
}

function pzSetTool(mode, color, size, opacity) {
  const E = window.pzEd;
  E.tool = { mode, color, size, opacity: opacity == null ? 1 : opacity };
  pzApplyMode();
  return true;
}

// ---- live image objects ----
// An inserted image lands immediately at its natural size on the current
// page, then behaves like a text box: drag to move, corner to resize,
// per-object opacity, ✕ / Delete to remove.

async function pzStageImage(id, url) {
  const blob = await (await fetch(url)).blob();
  return pzPlaceImage(id, blob);
}

async function pzStageImageB64(id, b64) {
  return pzPlaceImage(id, new Blob([pzB64(b64)]));
}

async function pzPlaceImage(id, blob) {
  const E = window.pzEd;
  const bmp = await createImageBitmap(blob);
  const n = E.current || 1;
  const p = E.pages[n];
  if (!p) return false;
  // Natural source pixels, capped to fit comfortably on the page.
  let w = bmp.width;
  let h = bmp.height;
  const cap = Math.min(1, (p.overlay.width * 0.6) / w, (p.overlay.height * 0.6) / h);
  w *= cap;
  h *= cap;
  const x = (p.overlay.width - w) / 2;
  const y = Math.max(12, (p.overlay.height - h) / 3);
  const rec = pzMakeImage(n, id, URL.createObjectURL(blob), x, y, w, h);
  pzPushHistory({ type: "image", page: n, obj: rec });
  pzSetTool("cursor", E.tool.color, E.tool.size, 1);
  rec.el.focus();
  return true;
}

function pzMakeImage(n, id, url, x, y, w, h) {
  const E = window.pzEd;
  const p = E.pages[n];
  const el = document.createElement("div");
  el.className = "pz-img";
  el.tabIndex = 0;
  el.style.left = x + "px";
  el.style.top = y + "px";
  el.style.width = w + "px";
  el.style.height = h + "px";
  const im = document.createElement("img");
  im.src = url;
  im.draggable = false;
  im.alt = "";
  const del = document.createElement("button");
  del.className = "pz-obj-del";
  del.textContent = "✕";
  del.title = "Delete image";
  const rs = document.createElement("div");
  rs.className = "pz-obj-resize";
  rs.title = "Resize (hold Shift for proportional)";
  const op = document.createElement("input");
  op.type = "range";
  op.min = "10";
  op.max = "100";
  op.value = "100";
  op.className = "pz-obj-opacity";
  op.title = "Opacity";
  el.append(im, del, rs, op);
  const rec = { el, id, x, y, w, h, opacity: 1 };
  op.addEventListener("pointerdown", (e) => e.stopPropagation());
  op.addEventListener("input", () => {
    rec.opacity = op.value / 100;
    im.style.opacity = rec.opacity;
  });
  del.addEventListener("pointerdown", (e) => e.stopPropagation());
  del.addEventListener("click", () => pzDeleteImage(n, rec, true));
  rs.addEventListener("pointerdown", (e) => pzImageResize(e, n, rec));
  el.addEventListener("pointerdown", (e) => pzImagePointer(e, n, rec));
  el.addEventListener("keydown", (e) => {
    if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      pzDeleteImage(n, rec, true);
    }
  });
  p.wrap.insertBefore(el, p.overlay);
  (E.images[n] = E.images[n] || []).push(rec);
  return rec;
}

function pzDeleteImage(n, rec, hist) {
  const E = window.pzEd;
  rec.el.remove();
  const arr = E.images[n] || [];
  const i = arr.indexOf(rec);
  if (i >= 0) arr.splice(i, 1);
  if (hist) pzPushHistory({ type: "image-del", page: n, obj: rec });
}

function pzImagePointer(ev, n, rec) {
  const E = window.pzEd;
  if (E.tool.mode !== "cursor" && E.tool.mode !== "text") return;
  ev.preventDefault();
  rec.el.focus();
  const startX = ev.clientX;
  const startY = ev.clientY;
  const origX = rec.x;
  const origY = rec.y;
  const onMove = (mv) => {
    rec.x = origX + (mv.clientX - startX);
    rec.y = origY + (mv.clientY - startY);
    rec.el.style.left = rec.x + "px";
    rec.el.style.top = rec.y + "px";
  };
  const onUp = () => {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
  };
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
}

function pzImageResize(ev, n, rec) {
  ev.preventDefault();
  ev.stopPropagation();
  const startX = ev.clientX;
  const startY = ev.clientY;
  const origW = rec.w;
  const origH = rec.h;
  const onMove = (mv) => {
    let w = Math.max(16, origW + (mv.clientX - startX));
    let h = Math.max(16, origH + (mv.clientY - startY));
    if (mv.shiftKey) {
      // Proportional: scale both axes by the dominant drag direction.
      const s = Math.max(w / origW, h / origH);
      w = Math.max(16, origW * s);
      h = Math.max(16, origH * s);
    }
    rec.w = w;
    rec.h = h;
    rec.el.style.width = rec.w + "px";
    rec.el.style.height = rec.h + "px";
  };
  const onUp = () => {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
  };
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
}

// ---- undo / redo ----

function pzUndo() {
  const E = window.pzEd;
  const last = E.history.pop();
  if (!last) return false;
  const n = last.page;
  if (last.type === "stroke") {
    const obj = (E.strokes[n] || []).pop();
    E.redo.push({ ...last, payload: obj });
  } else if (last.type === "image") {
    pzDeleteImage(n, last.obj, false);
    E.redo.push(last);
  } else if (last.type === "image-del") {
    E.pages[n].wrap.insertBefore(last.obj.el, E.pages[n].overlay);
    (E.images[n] = E.images[n] || []).push(last.obj);
    E.redo.push(last);
  } else if (last.type === "text") {
    pzRemoveText(n, last.obj, false);
    E.redo.push(last);
  } else if (last.type === "retype") {
    pzRemoveText(n, last.obj.box, false);
    const arr = E.rects[n] || [];
    const i = arr.indexOf(last.obj.rect);
    if (i >= 0) arr.splice(i, 1);
    if (last.obj.rect.spanEl) last.obj.rect.spanEl.style.visibility = "";
    E.redo.push(last);
  } else if (last.type === "redact") {
    pzDeleteRedact(n, last.obj, false);
    E.redo.push(last);
  } else if (last.type === "redact-del") {
    E.pages[n].wrap.insertBefore(last.obj.el, E.pages[n].overlay);
    (E.redacts[n] = E.redacts[n] || []).push(last.obj);
    E.redo.push(last);
  }
  pzRedraw(n);
  return true;
}

function pzRedo() {
  const E = window.pzEd;
  const last = E.redo.pop();
  if (!last) return false;
  const n = last.page;
  if (last.type === "stroke") {
    (E.strokes[n] = E.strokes[n] || []).push(last.payload);
    E.history.push({ type: "stroke", page: n });
  } else if (last.type === "image") {
    E.pages[n].wrap.insertBefore(last.obj.el, E.pages[n].overlay);
    (E.images[n] = E.images[n] || []).push(last.obj);
    E.history.push(last);
  } else if (last.type === "image-del") {
    pzDeleteImage(n, last.obj, false);
    E.history.push(last);
  } else if (last.type === "text") {
    const rec = last.obj;
    E.pages[n].wrap.insertBefore(rec.el, E.pages[n].overlay);
    (E.texts[n] = E.texts[n] || []).push(rec);
    E.history.push(last);
  } else if (last.type === "retype") {
    const { box, rect } = last.obj;
    if (rect.spanEl) rect.spanEl.style.visibility = "hidden";
    (E.rects[n] = E.rects[n] || []).push(rect);
    E.pages[n].wrap.insertBefore(box.el, E.pages[n].overlay);
    (E.texts[n] = E.texts[n] || []).push(box);
    E.history.push(last);
  } else if (last.type === "redact") {
    E.pages[n].wrap.insertBefore(last.obj.el, E.pages[n].overlay);
    (E.redacts[n] = E.redacts[n] || []).push(last.obj);
    E.history.push(last);
  } else if (last.type === "redact-del") {
    pzDeleteRedact(n, last.obj, false);
    E.history.push(last);
  }
  pzRedraw(n);
  return true;
}

// ---- export ----

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
      opacity: s.opacity == null ? 1 : s.opacity,
      points: s.points.map(([x, y]) => [x / p.scale, p.pdfH - y / p.scale]),
    }));
    const images = (E.images[n] || []).map((im) => ({
      id: im.id,
      opacity: im.opacity == null ? 1 : im.opacity,
      rect: [
        im.x / p.scale,
        p.pdfH - (im.y + im.h) / p.scale,
        im.w / p.scale,
        im.h / p.scale,
      ],
    }));
    const texts = (E.texts[n] || [])
      .filter((t) => t.el.innerText.trim())
      .map((t) => ({
        text: t.el.innerText.replace(/\n+$/, ""),
        color: t.color,
        size: t.size / p.scale,
        bold: !!t.bold,
        x: t.x / p.scale,
        y: p.pdfH - (t.y + t.size * 0.85) / p.scale,
      }));
    const rects = (E.rects[n] || []).map((r) => ({
      rect: [
        r.x / p.scale,
        p.pdfH - (r.y + r.h) / p.scale,
        r.w / p.scale,
        r.h / p.scale,
      ],
      color: [255, 255, 255],
    }));
    const redacts = (E.redacts[n] || []).map((r) => ({
      rect: [
        r.x / p.scale,
        p.pdfH - (r.y + r.h) / p.scale,
        r.w / p.scale,
        r.h / p.scale,
      ],
    }));
    if (strokes.length || images.length || texts.length || rects.length || redacts.length)
      out.push({ page: n, strokes, images, texts, rects, redacts });
  }
  return out;
}

// ---- page rasterization (PDF → PNG export) ----

// Render every page of the CURRENT working document to a PNG at `mult`×
// the page's natural resolution and return them as base64 strings. The
// caller bakes pending edits first, so what you see is what exports.
async function pzExportPages(mult) {
  const E = window.pzEd;
  const out = [];
  for (let n = 1; n <= E.doc.numPages; n++) {
    const page = await E.doc.getPage(n);
    const vp = page.getViewport({ scale: mult || 2 });
    const canvas = document.createElement("canvas");
    canvas.width = vp.width;
    canvas.height = vp.height;
    await page.render({ canvasContext: canvas.getContext("2d"), viewport: vp })
      .promise;
    const blob = await new Promise((res) => canvas.toBlob(res, "image/png"));
    const b64 = await new Promise((res) => {
      const fr = new FileReader();
      fr.onload = () => res(fr.result.split(",", 2)[1]);
      fr.readAsDataURL(blob);
    });
    out.push(b64);
  }
  return out;
}
