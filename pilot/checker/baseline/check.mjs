// baseline/check.mjs — library behind bin/check-baseline
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { buildB1, buildB2, cleanup } from './build.mjs';
import { launchBrowser } from './probe-browser.mjs';
import { loadSpec, runSpec } from '../core/runner.mjs';

export function serve(dir) {
  const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml' };
  const srv = http.createServer((q, s) => {
    let p = decodeURIComponent(q.url.split('?')[0]); if (p === '/') p = '/index.html';
    try { const f = path.join(dir, p); if (!f.startsWith(dir)) throw 0; const b = fs.readFileSync(f); s.setHeader('content-type', types[path.extname(f)] || 'application/octet-stream'); s.end(b); }
    catch { s.statusCode = 404; s.end('nf'); }
  });
  return new Promise(r => srv.listen(0, '127.0.0.1', () => r({ origin: 'http://127.0.0.1:' + srv.address().port, close: () => srv.close() })));
}

export async function build(stack, file) {
  return stack === 'B1' ? buildB1(file) : buildB2(file);
}

export async function checkBaseline(stack, id, file, { only = null, keep = false } = {}) {
  const spec = await loadSpec(id);
  const b = await build(stack, file);
  const out = { stack, id, build: { ok: b.ok, errors: b.errors, tsc: b.tsc }, results: [], blocked: [], log: [] };
  if (!b.ok && (b.kind === 'B1' || !b.bundled)) { if (!keep) cleanup(b); return out; } // cannot load anything
  const srv = await serve(b.dir);
  const br = await launchBrowser(srv.origin);
  const driver = { fresh: () => br.newProbe(), newSession: () => br.newProbe({ sameContext: true }), baseUrl: srv.origin };
  try {
    const r = await runSpec(spec, driver, { only });
    out.results = r.results; out.log = r.log; out.blocked = [...new Set(br.blocked)];
    out.spec = spec;
  } finally { await br.close(); srv.close(); if (!keep) cleanup(b); }
  return out;
}

/** compact verdict text. mode 'feedback' = exactly what an agent may see (G2 §4.4): ids + standardised text only. */
export function formatVerdict(out, { verbose = false } = {}) {
  const lines = [];
  const aRes = out.results.filter(r => r.kind === 'A');
  const failed = aRes.filter(r => !r.pass);
  const buildFail = !out.build.ok;
  const h = out.results.filter(r => r.kind === 'H').length;
  const pass = !buildFail && failed.length === 0 && aRes.length > 0;
  lines.push(`RESULT ${pass ? 'PASS' : 'FAIL'} ${out.stack} ${out.id} | A-items passed ${aRes.length - failed.length}/${aRes.length} | H-items not judged ${h}`);
  if (buildFail) {
    lines.push('FAIL BUILD: Not met: the program must build (type-check, bundle and Tailwind compile) without errors.');
    if (verbose) { for (const e of out.build.errors) lines.push('  ' + e); for (const t of out.build.tsc.slice(0, 5)) lines.push(`  ${t.file}(${t.line},${t.col}) ${t.code}: ${t.msg}`); }
  }
  for (const r of failed) {
    lines.push(`FAIL ${r.id}: ${r.feedback}`);
    if (verbose) lines.push(`  detail: ${r.detail}${r.unevaluable ? ' [unevaluable]' : ''}`);
  }
  if (verbose && out.blocked.length) lines.push('  blocked network requests: ' + out.blocked.join(', '));
  if (verbose) for (const l of out.log) lines.push('  log: ' + l);
  return { text: lines.join('\n'), pass, failedIds: [...(buildFail ? ['BUILD'] : []), ...failed.map(r => r.id)] };
}
