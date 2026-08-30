// ffmpeg.wasm shim for the video tools (ADR-0010).
//
// The heavy lifting happens in a Web Worker spawned by the bundled
// @ffmpeg/ffmpeg wrapper; this file only moves bytes in and out and turns
// nonzero exits into readable errors. Everything loads same-origin from
// /ffmpeg/ (unhashed on purpose: the wrapper resolves its worker chunk
// relative to its own URL, which dx asset hashing would break — see
// scripts/fetch-ffmpeg.sh). Never a CDN, so the offline and
// no-third-party promises hold.
//
// The single-threaded core is deliberate: the multithreaded build needs
// SharedArrayBuffer and therefore cross-origin isolation headers, which
// ADR-0004 already rejected for the engine worker.

const V = (window.pzVid = window.pzVid || { ff: null, logs: [] });

function pzVidScript(src) {
  return new Promise((res, rej) => {
    const el = document.createElement("script");
    el.src = src;
    el.onload = res;
    el.onerror = () => rej(new Error(`could not load ${src}`));
    document.head.appendChild(el);
  });
}

async function pzVidInit() {
  if (V.ff) return true;
  if (typeof FFmpegWASM === "undefined") {
    await pzVidScript("/ffmpeg/ffmpeg.js");
  }
  const ff = new FFmpegWASM.FFmpeg();
  ff.on("log", (l) => {
    V.logs.push(l.message);
    if (V.logs.length > 40) V.logs.shift();
  });
  await ff.load({
    coreURL: "/ffmpeg/ffmpeg-core.js",
    wasmURL: "/ffmpeg/ffmpeg-core.wasm",
  });
  V.ff = ff;
  return true;
}

function pzVidB64(b64) {
  const bin = atob(b64);
  const arr = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
  return arr;
}

// Run `argSets` (one or more full ffmpeg invocations — the GIF palette
// recipe needs two) against an input written as `inName`, then read and
// return `outName` as base64. Scratch files are always cleaned up; the
// worker itself stays warm so a second run skips the load.
async function pzVidRun(inputUrl, inName, argSets, outName, scratch) {
  const ff = V.ff;
  const bytes =
    inputUrl.startsWith("b64:")
      ? pzVidB64(inputUrl.slice(4))
      : new Uint8Array(await (await fetch(inputUrl)).arrayBuffer());
  await ff.writeFile(inName, bytes);
  try {
    for (const args of argSets) {
      V.logs.length = 0;
      const ret = await ff.exec(args);
      if (ret !== 0) {
        const tail = V.logs.slice(-6).join("\n");
        throw new Error(`ffmpeg failed (exit ${ret}):\n${tail}`);
      }
    }
    const out = await ff.readFile(outName);
    if (!out || out.length === 0) {
      throw new Error("ffmpeg produced no output");
    }
    let b64 = "";
    const CHUNK = 0x8000;
    for (let i = 0; i < out.length; i += CHUNK) {
      b64 += String.fromCharCode.apply(null, out.subarray(i, i + CHUNK));
    }
    return btoa(b64);
  } finally {
    for (const f of [inName, outName, ...(scratch || [])]) {
      try {
        await ff.deleteFile(f);
      } catch {
        // scratch file may not exist if an early step failed
      }
    }
  }
}
