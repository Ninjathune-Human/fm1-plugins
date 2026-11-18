// Rendu de référence : le même scénario que ../spike/drive.c, joué par le module WebAssembly (celui du site).
// node ref.mjs ref.wasm note [ACCORD=1] [BLOCS=n]   -> somme de contrôle sur les bits des flottants, comme drive.c
import { readFileSync } from "node:fs";

const [, , file, note] = process.argv;
const blocs = process.env.BLOCS ? +process.env.BLOCS : Math.floor((44100 * 2) / 256), boot = 100;
let ex, now = 0, state = "run";
const RUN = 0, UNWOUND = 1, REWINDING = 2, ENDED = 3; let st = ENDED, data = 0;
const dv = () => new DataView(ex.memory.buffer);
const imports = {
  env: {
    emscripten_get_now: () => now,
    emscripten_sleep: () => {
      if (st === REWINDING) { ex.asyncify_stop_rewind(); st = RUN; return; }
      new Int32Array(ex.memory.buffer, data, 1)[0] = data + 8;
      ex.asyncify_start_unwind(data); st = UNWOUND;
    },
  },
  wasi_snapshot_preview1: new Proxy({
    fd_write: (fd, iov, n, out) => { let len = 0; for (let i = 0; i < n; i++) len += dv().getUint32(iov + 8 * i + 4, true); dv().setUint32(out, len, true); return 0; },
    environ_sizes_get: (a, b) => { dv().setUint32(a, 0, true); dv().setUint32(b, 0, true); return 0; },
    proc_exit: () => { st = ENDED; throw new Error("fin"); },
  }, { get: (t, k) => (k in t ? t[k] : () => 0) }),
};
const run = () => { try { ex.__main_void(); } catch { st = ENDED; return; } if (st === UNWOUND) ex.asyncify_stop_unwind(); else st = ENDED; };
const { instance } = await WebAssembly.instantiate(readFileSync(file), imports);
ex = instance.exports; if (ex._initialize) ex._initialize();
const PILE = 1 << 17; data = ex.malloc(8 + PILE);
new Int32Array(ex.memory.buffer, data, 2).set([data + 8, data + 8 + PILE]);
st = RUN; run();                                   // démarrage, comme le worklet
let sum = 0n, peak = 0; const M = (1n << 64n) - 1n;
for (let b = 0; b < boot + blocs; b++) {
  if (b === boot) {
    ex.web_midi(0x90, +note, 100);
    if (process.env.ACCORD) for (let j = 1; j < 12; j++) ex.web_midi(0x90, +note + 2 * j, 100);
  }
  if (process.env.SCENE) {
    if (b === boot + 50) { ex.web_enc(0, 3); ex.web_enc(1, -2); }
    if (b === boot + 60) ex.web_btn(2, 1);
    if (b === boot + 70) ex.web_btn(2, 0);
  }
  now += (256 * 1000) / 44100;
  if (st === UNWOUND) { ex.asyncify_start_rewind(data); st = REWINDING; run(); }
  const o = new Float32Array(ex.memory.buffer, ex.web_audio(), 512), u = new Uint32Array(o.buffer, o.byteOffset, 512);
  for (let i = 0; i < 512; i++) { sum = (sum * 1099511628211n + BigInt(u[i])) & M; if (o[i] > peak) peak = o[i]; }
}
let f = 0n; const px = new Uint16Array(ex.memory.buffer, ex.web_fb(), 240 * 240);
for (let i = 0; i < 240 * 240; i++) f = (f * 1099511628211n + BigInt(px[i])) & M;
console.log(`instance 0 écran ${f.toString(16).padStart(16, "0")}`);
console.log(`instance 0 note ${note} : somme ${sum.toString(16).padStart(16, "0")} crête ${peak.toFixed(3)}`);
