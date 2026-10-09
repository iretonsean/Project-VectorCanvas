// baseline/make-validation-md.mjs — renders checker_validation.md from tmp/validation_rows.jsonl (written by validate.mjs)
import fs from 'node:fs';
import path from 'node:path';
import { MUTATIONS } from '../examples/mutations.mjs';
const ROOT = path.join(path.dirname(new URL(import.meta.url).pathname), '..');
const rows = fs.readFileSync(path.join(ROOT, 'tmp', 'validation_rows.jsonl'), 'utf8').trim().split('\n').map(l => JSON.parse(l));
const meta = JSON.parse(fs.readFileSync(path.join(ROOT, 'tmp', 'validation_meta.json'), 'utf8'));
const IDS = ['L01', 'L02', 'L03', 'L04', 'L05', 'L06', 'L07', 'L08', 'L09', 'L10', 'G01', 'G02', 'G03', 'G04', 'G05'];
const L = ['# Checker validation (B4) — baseline stack', '',
  `Run: ${meta.date} · host=container-xeon4 (Chromium 141.0.7390.37 headless via Playwright 1.56.0, offline, 127.0.0.1 only) · load average at start ${meta.load} · command \`node baseline/validate.mjs\` (raw rows: \`tmp/validation_rows.jsonl\`, log: \`tmp/validate_final.log\`).`, '',
  'Method: for each specimen and stack (B1 = HTML+Tailwind+JS, B2 = React+TypeScript+Tailwind with MUI available) one hand-written **passing** example must PASS (all specimen A items + U1–U5, H items not judged) and every hand-written **failing** example (a mutation of the passing one, `examples/mutations.mjs`) must FAIL **and** fail the item(s) it was written to break.',
  '**Scope limit (stated):** this validates the checker on 15 specimens × my own examples. It does not measure the false-fail rate on real agent output (no agent output exists yet; no pilot run was made). The PVC column is pending (§PVC).', '',
  '| Specimen | stack | passing example | failing examples that failed the intended item | all OK |', '|---|---|---|---|---|'];
let tot = 0, okn = 0;
for (const id of IDS) for (const stack of ['B1', 'B2']) {
  const rs = rows.filter(r => r.id === id && r.stack === stack);
  const p = rs.find(r => r.kind === 'pass'); const f = rs.filter(r => r.kind === 'fail');
  const fo = f.filter(r => r.ok).length; const allok = rs.length > 0 && rs.every(r => r.ok);
  tot += rs.length; okn += rs.filter(r => r.ok).length;
  L.push(`| ${id} | ${stack} | ${p ? (p.ok ? 'PASS ✔' : '**FAIL ✘**') : 'missing'} | ${fo}/${f.length} | ${allok ? '✔' : '**✘**'} |`);
}
L.push('', `**Totals: ${okn}/${tot} example runs behaved as required.**`, '', '## Every example run', '', '| stack | specimen | example | must fail | failed items observed | as required |', '|---|---|---|---|---|---|');
for (const r of rows) L.push(`| ${r.stack} | ${r.id} | ${r.name} | ${r.kind === 'pass' ? '— (must pass)' : r.expect.join(', ')} | ${r.err ? 'ERROR ' + r.err : (r.failed.join(', ') || 'none (PASS)')} | ${r.ok ? '✔' : '✘'} |`);
const aItems = {};
for (const id of IDS) { const sp = (await import(`../core/specs/${id}.mjs`)).default; aItems[id] = sp.items.filter(i => i.kind === 'A').map(i => i.id); }
const covered = new Set(MUTATIONS.flatMap(m => m.expect));
const unc = IDS.flatMap(id => aItems[id]).filter(x => !covered.has(x));
L.push('', '## Coverage', '', `Specimen A items with a mutation written to break exactly that item: ${IDS.flatMap(id => aItems[id]).length - unc.length}/${IDS.flatMap(id => aItems[id]).length}${unc.length ? ' (not asserted: ' + unc.join(', ') + ')' : ''}.`,
  `Universal items asserted by a dedicated mutation: ${['U1', 'U2', 'U3', 'U4', 'U5'].filter(u => covered.has(u)).join(', ')}; the rest (${['U1', 'U2', 'U3', 'U4', 'U5'].filter(u => !covered.has(u)).join(', ') || 'none'}) were only observed as side effects (see "failed items observed").`,
  'BUILD (B2 `tsc` type error) asserted by `L01/type-error`.', '',
  '## PVC checker', '', 'Status: **pending.** `pvc/check-pvc` and the protocol client are written against `PVC_CHECKER_NEEDS.md`; they were exercised only against `pvc/mock-probe.mjs` (the Chromium probe wrapped in the pvc-probe protocol): L01 pass→PASS, L01 avatar-square→FAIL L01.2, G01 pass→PASS (including `new_session` and keyboard replay), and a fake validator-failure probe → `FAIL BUILD` with the diagnostics. That is a wiring test of the checker, **not** evidence about PVC. Validation on PVC examples starts when `pvc-probe` and the v0 grammar for the pilot constructs exist.', '');
fs.writeFileSync(path.join(ROOT, 'checker_validation.md'), L.join('\n'));
console.log('wrote checker_validation.md', okn, '/', tot);
