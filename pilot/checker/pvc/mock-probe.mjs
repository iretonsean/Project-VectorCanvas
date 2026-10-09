#!/usr/bin/env node
// pvc/mock-probe.mjs — PROTOCOL CONFORMANCE MOCK, NOT PVC. Implements the `pvc-probe` JSON-lines protocol on top of the
// Chromium baseline probe, for a B1 .html file, so that check-pvc / probe-pvc.mjs / the shared evaluator can be exercised end to end
// before the real pvc-runtime exists.   usage: node mock-probe.mjs <file.html>
import readline from 'node:readline';
import { buildB1, cleanup } from '../baseline/build.mjs';
import { serve } from '../baseline/check.mjs';
import { launchBrowser } from '../baseline/probe-browser.mjs';

const file = process.argv[2];
let b, srv, br, p;
async function start(sameContext) { p = await br.newProbe({ sameContext }); }
const rl = readline.createInterface({ input: process.stdin });
const out = o => process.stdout.write(JSON.stringify(o) + '\n');
let chain = Promise.resolve();
rl.on('line', line => { chain = chain.then(() => handle(line)); });
rl.on('close', async () => { try { await br?.close(); srv?.close(); b && cleanup(b); } catch (e) { /* */ } process.exit(0); });
async function handle(line) {
  const r = JSON.parse(line);
  try {
    switch (r.op) {
      case 'load': b = buildB1(file); srv = await serve(b.dir); br = await launchBrowser(srv.origin); await start(false); return out({ ok: true, diagnostics: [] });
      case 'snapshot': return out({ ok: true, snapshot: await p.snapshot({ w: r.viewport?.[0] ?? 1280, h: r.viewport?.[1] ?? 800, scale: r.text_scale ?? 1, dir: r.direction ?? 'ltr' }) });
      case 'click': await p.click(r.id); return out({ ok: true, hit: r.id });
      case 'type': await p.type(r.id, r.text, { replace: !!r.replace }); return out({ ok: true });
      case 'type_focused': await p.typeFocused(r.text); return out({ ok: true });
      case 'key': return out({ ok: true, focused: await p.key(r.key) });
      case 'focused': return out({ ok: true, focused: await p.focused() });
      case 'blur_all': await p.blurAll(); return out({ ok: true });
      case 'new_session': await p.close(); await start(true); return out({ ok: true });
      case 'reset': await p.close(); await start(false); return out({ ok: true });
      default: return out({ ok: false, error: 'unknown op ' + r.op });
    }
  } catch (e) { out({ ok: false, error: String(e.message || e).split('\n')[0] }); }
}
