// D1 cross-target check: wasm32 build of pvc-ir (run under node) vs native build, byte for byte.
// usage: node run_d1.mjs <wasm> <native-dir-with-L01.canonical.json,L01.pvca,...> <specimens-dir>
import fs from 'node:fs';
import crypto from 'node:crypto';
const [wasmPath, nativeDir, specDir] = process.argv.slice(2);
const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasmPath), {});
const x = instance.exports;
const mem = () => new Uint8Array(x.memory.buffer);
function put(bytes) { const p = x.alloc(bytes.length); mem().set(bytes, p); return p; }
function out(len) { const p = x.out_ptr(); return Buffer.from(mem().slice(p, p + len)); }
const sha = b => crypto.createHash('sha256').update(b).digest('hex');
let fail = 0;
const line = (s) => console.log(s);
const refsDir = process.argv[5];
const jobs = [['L01', specDir], ['F01', specDir]];
if (refsDir) for (const n of ['L01','L02','L03','L04','L05','L06','L07','L08','L09','L10','G01','G02','G03','G04','G05']) jobs.push([n, refsDir, 'ref-']);
for (const [name0, dir, prefix = ''] of jobs) {
  const name = prefix + name0;
  const src = fs.readFileSync(`${dir}/${name0}.pvc`);
  const n = x.compile_canonical(put(src), src.length);
  if (n === 0) { line(`FAIL ${name}: wasm compile error: ${out(0).toString()}`); fail++; continue; }
  const canonWasm = out(n);
  const canonNative = fs.readFileSync(`${nativeDir}/${name}.canonical.json`);
  const same = Buffer.compare(canonWasm, canonNative) === 0;
  line(`${same ? 'OK  ' : 'FAIL'} ${name} canonical: wasm32 sha256=${sha(canonWasm)} native sha256=${sha(canonNative)} bytes=${canonWasm.length}`);
  if (!same) fail++;
  // in-wasm sha256 agrees with node's
  const h = x.sha256_hex(put(canonWasm), canonWasm.length); const hw = out(h).toString();
  if (hw !== sha(canonWasm)) { line(`FAIL ${name}: in-wasm sha256 ${hw} != ${sha(canonWasm)}`); fail++; }
  // signed artifact bytes (Ed25519 is deterministic) identical
  const m = x.compile_signed(put(src), src.length);
  const artWasm = out(m);
  const artNative = fs.readFileSync(`${nativeDir}/${name}.pvca`);
  const sameArt = Buffer.compare(artWasm, artNative) === 0;
  line(`${sameArt ? 'OK  ' : 'FAIL'} ${name} signed artifact: wasm32 sha256=${sha(artWasm)} native sha256=${sha(artNative)}`);
  if (!sameArt) fail++;
  // wasm verifies the native-built artifact; refuses a tampered one
  const okNative = x.verify(put(artNative), artNative.length);
  const t = Buffer.from(artNative); t[t.length - 5] ^= 1;
  const okTamper = x.verify(put(t), t.length);
  line(`${okNative === 1 && okTamper === 0 ? 'OK  ' : 'FAIL'} ${name} wasm32 load_signed: native artifact accepted=${okNative} tampered accepted=${okTamper}`);
  if (!(okNative === 1 && okTamper === 0)) fail++;
}
line(fail === 0 ? 'D1 CROSS-TARGET: PASS' : `D1 CROSS-TARGET: FAIL (${fail})`);
process.exit(fail ? 1 : 0);
