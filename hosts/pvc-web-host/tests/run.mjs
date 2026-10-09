// Scenario runner: node tests/run.mjs <scenario> [--variant raf|sync] [--mirror on|off] [--dsf N] [--out DIR]
import path from 'node:path';
import { open, writeJson, requestAudit, HOST_LABEL, SCOPE_LABELS, ROOT } from './lib.mjs';
import { SCENARIOS } from './scenarios.mjs';

const argv = process.argv.slice(2);
const name = argv[0];
const opt = (k, d) => { const i = argv.indexOf('--' + k); return i >= 0 ? argv[i + 1] : d; };
if (!SCENARIOS[name]) { console.error('scenarios:', Object.keys(SCENARIOS).join(', ')); process.exit(2); }
const outDir = path.resolve(opt('out', path.join(ROOT, 'out', name)));
const sc = SCENARIOS[name];
const openOpts = { scenario: name, variant: opt('variant', 'raf'), mirror: opt('mirror', 'on'), dsf: Number(opt('dsf', 1)), specimen: opt('specimen', sc.specimen || 'F01'), locale: opt('locale', sc.locale || 'en'), ...(sc.open || {}) };
const s = await open(openOpts);
let result;
try {
  result = await sc.run(s, { outDir, opt });
} finally {
  const audit = requestAudit(s.requests, s.base);
  writeJson(path.join(outDir, 'requests.json'), { requests: s.requests, audit });
  writeJson(path.join(outDir, 'run_meta.json'), { host_label: HOST_LABEL, labels: SCOPE_LABELS, scenario: name, openOpts, cmd: process.argv.join(' '), date: new Date().toISOString(), loadavg: (await import('node:fs')).readFileSync('/proc/loadavg', 'utf8').trim(), console: s.consoleMsgs, goto_to_ready_ms: s.goto_to_ready_ms });
  await s.close();
  if (result) writeJson(path.join(outDir, 'result.json'), result);
  console.log(JSON.stringify({ scenario: name, out: outDir, requestAuditPass: audit.pass, result: result && result.summary }, null, 1));
  if (result && result.pass === false) process.exitCode = 1;
  if (!audit.pass) process.exitCode = 1;
}
