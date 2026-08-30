// Folder drag-and-drop for the tool dropzones.
//
// Dropping a directory gives the page nothing through `dataTransfer.files`
// — its contents are only reachable by walking `webkitGetAsEntry()`
// trees, which the Dioxus event layer doesn't expose. This shim watches
// the drop in the capture phase; when (and only when) a directory is
// among the dropped items it takes over: walks the tree, turns every file
// into a blob: URL, and queues the list for the Rust side, which fetches
// the bytes and feeds the normal file list. Plain file drops are left
// untouched for the existing Dioxus handler.
//
// Nothing here attaches until Rust calls pzDropInit() — only multi-file
// tool pages on the web build do, so single-file tools and native
// platforms keep their exact current behavior.

const D = (window.pzDropState = window.pzDropState || {
  queue: [],
  waiter: null,
  attached: false,
});

// Desktop-OS clutter nobody means to zip.
const PZ_DROP_SKIP = new Set([".DS_Store", "Thumbs.db", "desktop.ini"]);

function pzDropPush(files) {
  if (D.waiter) {
    const w = D.waiter;
    D.waiter = null;
    w(files);
  } else {
    D.queue.push(files);
  }
}

// Rust awaits this in a loop (dioxus.send bridge in tool.rs).
function pzNextDrop() {
  return D.queue.length
    ? Promise.resolve(D.queue.shift())
    : new Promise((res) => (D.waiter = res));
}

// Flatten FileSystemEntry trees into [{name, url, size}]. Only duck-typed
// members are used (isFile/isDirectory/fullPath/file()/createReader()),
// so tests can exercise this with synthetic trees — a real drop gesture
// can't be simulated headless.
async function pzWalkEntries(entries) {
  const out = [];
  async function walk(entry) {
    if (entry.isFile) {
      const f = await new Promise((res, rej) => entry.file(res, rej));
      const name = (entry.fullPath || f.name).replace(/^\//, "");
      if (PZ_DROP_SKIP.has(name.split("/").pop())) return;
      out.push({ name, url: URL.createObjectURL(f), size: f.size });
    } else if (entry.isDirectory) {
      const reader = entry.createReader();
      // readEntries hands back batches (Chrome: 100 max) — one call is
      // NOT the whole directory; loop until an empty batch.
      for (;;) {
        const batch = await new Promise((res, rej) => reader.readEntries(res, rej));
        if (!batch.length) break;
        for (const e of batch) await walk(e);
      }
    }
  }
  for (const e of entries) await walk(e);
  out.sort((a, b) => a.name.localeCompare(b.name));
  return out;
}

// Shared by the drop listener and the tests.
async function pzIngestEntries(entries) {
  pzDropPush(await pzWalkEntries(entries));
}

function pzDropInit() {
  // A page may have been left mid-drop: anything still queued has no
  // consumer anymore — revoke the pinned bytes instead of surfacing
  // another page's files here.
  for (const batch of D.queue.splice(0)) {
    for (const f of batch) URL.revokeObjectURL(f.url);
  }
  D.waiter = null;
  if (D.attached) return true;
  D.attached = true;
  document.addEventListener(
    "drop",
    (e) => {
      // Only dropzones that opted in (multi-file tool pages render
      // data-dropdir) get intercepted — on the editor and single-file
      // tools this listener must stay inert, so gating lives in the
      // CURRENT page's DOM rather than in a stale global flag.
      if (
        !e.target ||
        !(e.target.closest && e.target.closest('.dropzone[data-dropdir="1"]'))
      ) {
        return;
      }
      const items = e.dataTransfer ? [...e.dataTransfer.items] : [];
      // getAsEntry must happen synchronously, before the event ends.
      const entries = items
        .map((i) => (i.webkitGetAsEntry ? i.webkitGetAsEntry() : null))
        .filter(Boolean);
      if (!entries.some((en) => en.isDirectory)) {
        return; // plain files: the Dioxus handler owns this drop
      }
      e.preventDefault();
      e.stopPropagation();
      pzIngestEntries(entries);
    },
    true,
  );
  return true;
}
