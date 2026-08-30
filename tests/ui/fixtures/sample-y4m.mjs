// Builds a tiny uncompressed video fixture in memory: YUV4MPEG2 (.y4m),
// the simplest container ffmpeg reads natively — a text header plus raw
// 4:2:0 frames, no codec involved. 8x8, 10 fps, 16 frames (1.6 s), with
// a moving luma gradient so encoded output isn't a trivial solid.
export function sampleY4m() {
  const W = 8, H = 8, FRAMES = 16;
  const header = new TextEncoder().encode(`YUV4MPEG2 W${W} H${H} F10:1 Ip A1:1 C420\n`);
  const frameMark = new TextEncoder().encode("FRAME\n");
  const ySize = W * H, cSize = (W / 2) * (H / 2);
  const parts = [header];
  for (let f = 0; f < FRAMES; f++) {
    parts.push(frameMark);
    const y = new Uint8Array(ySize);
    for (let r = 0; r < H; r++)
      for (let c = 0; c < W; c++)
        y[r * W + c] = (16 + ((r + c + f * 2) * 13)) % 220 + 16;
    const u = new Uint8Array(cSize).fill(96 + f * 4);
    const v = new Uint8Array(cSize).fill(160 - f * 4);
    parts.push(y, u, v);
  }
  const total = parts.reduce((n, p) => n + p.length, 0);
  const out = new Uint8Array(total);
  let off = 0;
  for (const p of parts) { out.set(p, off); off += p.length; }
  return Buffer.from(out);
}
