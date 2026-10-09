// pvc-web bootstrap. Fetches only same-origin static assets (wasm, artifact, key). No other network.
import init, { PvcWeb } from './pkg/pvc_web.js';

const q = new URLSearchParams(location.search);
const scenario = q.get('scenario') || 'manual';
const variant = q.get('variant') === 'sync' ? 'sync' : 'raf';
const mirrorOn = q.get('mirror') !== 'off';
const specimen = q.get('specimen') || 'F01';
const locale = q.get('locale') || 'en';
if (q.get('debug') === '1') document.body.classList.add('pvc-debug');

window.__pvcFrames = [];
window.__pvcMeta = { specimen, locale, scenario, navigator_gpu: typeof navigator.gpu };
window.__pvcReady = false;
const fail = (m) => { window.__pvcMeta.error = String(m); console.error('PVC boot failed:', m); };

// Assertion 1 (brief item 2): navigator.gpu hidden. The init script (Playwright addInitScript)
// does the hiding; the page only asserts it.
if (typeof navigator.gpu !== 'undefined') { fail('navigator.gpu is defined: WebGPU path not hidden'); throw new Error('navigator.gpu defined'); }

async function fetchOpt(url) {
  const r = await fetch(url, { cache: 'no-store' });
  if (!r.ok) return null;
  return new Uint8Array(await r.arrayBuffer());
}

try {
  const t0 = performance.now();
  await init();
  // overlay font = the runtime's bundled Latin face (the CJK face is fetched by the browser only when a glyph needs it)
  try { await document.fonts.load("16px 'DejaVu Sans'"); } catch (_) {}
  window.__pvcMeta.fonts_ready_ms = performance.now() - t0;
  window.__pvcMeta.wasm_init_ms = performance.now() - t0;
  const artName = q.get('artifact') || `${specimen}.signed`;
  const art = await fetchOpt(`artifacts/${artName}`);
  window.__pvcMeta.artifact = art ? `artifacts/${artName} (${art.length} bytes)` : 'not available';
  const pvc = new PvcWeb('pvc-canvas', 'pvc-mirror', art || new Uint8Array(), scenario, locale, variant === 'sync', mirrorOn);
  window.__pvc = pvc;
  // Assertion 2: a webgl2 context was obtained (PvcWeb.new would have thrown otherwise); record renderer.
  window.__pvcMeta.webgl2 = true;
  window.__pvcMeta.unmasked_renderer = pvc.renderer();

  const canvas = document.getElementById('pvc-canvas');
  const pos = (e) => { const r = canvas.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };
  canvas.addEventListener('pointermove', (e) => pvc.pointer('move', ...pos(e)));
  canvas.addEventListener('pointerdown', (e) => pvc.pointer('down', ...pos(e)));
  canvas.addEventListener('pointerup', (e) => pvc.pointer('up', ...pos(e)));
  // Keys while no overlay <input> has focus (Tab traversal, Enter/Space on buttons). Overlay inputs
  // handle their own keydown (overlay.rs).
  document.addEventListener('keydown', (e) => {
    if (e.target instanceof HTMLInputElement) return;
    if (e.key === 'Tab') { if (pvc.tab(e.shiftKey)) e.preventDefault(); return; }
    if (pvc.key(e.key, e.shiftKey, e.ctrlKey || e.metaKey, e.altKey)) e.preventDefault();
  });
  window.addEventListener('resize', () => pvc.resize(innerWidth, innerHeight, devicePixelRatio));

  // Golden-image hook for dev-render A7: pixels via readPixels (same task as the render) + display-list fingerprint.
  window.__pvcGoldenCapture = async () => {
    const g = pvc.golden();
    const c = document.createElement('canvas'); c.width = g.width; c.height = g.height;
    c.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(g.rgba.buffer, g.rgba.byteOffset, g.rgba.length), g.width, g.height), 0, 0);
    const url = c.toDataURL('image/png');
    window.__pvcGolden = { rgba_png_base64: url.slice(url.indexOf(',') + 1), dl_fingerprint: g.dl_fingerprint, width: g.width, height: g.height };
    return window.__pvcGolden;
  };

  const refreshRects = () => { try { window.__pvcRects = JSON.parse(pvc.rects_json()); } catch (_) {} };
  window.__pvcRectsRefresh = refreshRects;
  const poll = setInterval(() => { if (window.__pvcReady) { refreshRects(); clearInterval(poll); } }, 20);
} catch (e) { fail(e); }
