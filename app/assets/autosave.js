// Editor autosave storage (ADR-0013).
//
// Keeps the working document across a refresh so a long editing session
// isn't lost to an accidental F5. Everything stays on this device: the
// bytes go into IndexedDB, sealed by the Rust side with AES-256-GCM, and
// the key lives in localStorage — separate on purpose. Discarding wipes
// the key first, so even if the browser is lazy about reclaiming the
// blob on disk, what remains is unreadable (crypto-shredding: deleting
// 32 bytes is far more reliable than overwriting 50 MB).

const PZ_DB = "pz-editor";
const PZ_STORE = "doc";
const PZ_KEY = "pz-ed-key";
// Autosaves older than this are dropped unread on the next visit.
const PZ_MAX_AGE_MS = 24 * 60 * 60 * 1000;

function pzAutoDb() {
  return new Promise((res, rej) => {
    const req = indexedDB.open(PZ_DB, 1);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(PZ_STORE)) db.createObjectStore(PZ_STORE);
    };
    req.onsuccess = () => res(req.result);
    req.onerror = () => rej(req.error);
  });
}

function pzAutoTx(db, mode, fn) {
  return new Promise((res, rej) => {
    const tx = db.transaction(PZ_STORE, mode);
    const req = fn(tx.objectStore(PZ_STORE));
    tx.oncomplete = () => res(req ? req.result : undefined);
    tx.onerror = () => rej(tx.error);
    tx.onabort = () => rej(tx.error);
  });
}

// `url` is a blob: URL of the SEALED bytes; `keyHex` is the AES key.
async function pzAutosaveSave(url, name, keyHex) {
  try {
    const bytes = new Uint8Array(await (await fetch(url)).arrayBuffer());
    const db = await pzAutoDb();
    await pzAutoTx(db, "readwrite", (s) =>
      s.put({ name, bytes, savedAt: Date.now() }, "current"),
    );
    db.close();
    localStorage.setItem(PZ_KEY, keyHex);
    return true;
  } catch (e) {
    // Private-browsing quota errors etc. must never break editing.
    return false;
  }
}

// Metadata only — cheap enough to call on every editor mount.
async function pzAutosavePeek() {
  try {
    if (!localStorage.getItem(PZ_KEY)) return null;
    const db = await pzAutoDb();
    const rec = await pzAutoTx(db, "readonly", (s) => s.get("current"));
    db.close();
    if (!rec) return null;
    const age = Date.now() - rec.savedAt;
    if (age > PZ_MAX_AGE_MS) {
      await pzAutosaveClear();
      return null;
    }
    return { name: rec.name, ageSeconds: Math.floor(age / 1000) };
  } catch {
    return null;
  }
}

// Returns { url, key } — the sealed bytes as a blob: URL for Rust to
// fetch, plus the key to open them with.
async function pzAutosaveLoad() {
  const key = localStorage.getItem(PZ_KEY);
  if (!key) return null;
  const db = await pzAutoDb();
  const rec = await pzAutoTx(db, "readonly", (s) => s.get("current"));
  db.close();
  if (!rec) return null;
  const url = URL.createObjectURL(new Blob([rec.bytes]));
  return { url, key, name: rec.name };
}

async function pzAutosaveClear() {
  // Key first: after this the stored bytes are unreadable even if the
  // delete below fails or the browser keeps the blob around.
  localStorage.removeItem(PZ_KEY);
  try {
    const db = await pzAutoDb();
    await pzAutoTx(db, "readwrite", (s) => s.delete("current"));
    db.close();
  } catch {
    // Nothing recoverable to do; the key is already gone.
  }
  return true;
}
