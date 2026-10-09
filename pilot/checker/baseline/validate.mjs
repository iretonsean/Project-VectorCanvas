// baseline/validate.mjs — validates the baseline checker on hand-written passing + failing examples (B1 and B2).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { checkBaseline, formatVerdict } from './check.mjs';
import { MUTATIONS } from '../examples/mutations.mjs';

const IDS = ['L01', 'L02', 'L03', 'L04', 'L05', 'L06', 'L07', 'L08', 'L09', 'L10', 'G01', 'G02', 'G03', 'G04', 'G05'];
const EX = path.join(path.dirname(new URL(import.meta.url).pathname), '..', 'examples');
const filter = process.argv[2] ? new RegExp(process.argv[2]) : null;
const rows = [];
const t0 = Date.now();
for (const id of IDS) for (const stack of ['B1', 'B2']) {
  const ext = stack === 'B1' ? 'html' : 'tsx';
  const jobs = [{ kind: 'pass', file: `${stack}/${id}.pass.${ext}`, expect: [] }];
  for (const m of MUTATIONS.filter(m => m.id === id)) {
    if (stack === 'B1' && m.name === 'type-error') continue;
    jobs.push({ kind: 'fail', name: m.name, file: `${stack}/${id}.fail-${m.name}.${ext}`, expect: m.expect });
  }
  for (const j of jobs) {
    if (filter && !filter.test(`${stack}/${id}/${j.name ?? 'pass'}`)) continue;
    let out, v, err = null;
    try { out = await checkBaseline(stack, id, path.join(EX, j.file)); v = formatVerdict(out, { verbose: true }); }
    catch (e) { err = String(e.message || e).split('\n')[0]; }
    let ok;
    if (err) ok = false;
    else if (j.kind === 'pass') ok = v.pass;
    else ok = !v.pass && j.expect.every(x => v.failedIds.includes(x));
    rows.push({ stack, id, kind: j.kind, name: j.name ?? 'pass', file: j.file, expect: j.expect, failed: v?.failedIds ?? [], ok, err, detail: out ? out.results.filter(r => r.kind === 'A' && !r.pass).map(r => `${r.id}: ${r.detail}`).join(' | ').slice(0, 300) : '' });
    fs.appendFileSync(path.join(EX, '..', 'tmp', 'validation_rows.jsonl'), JSON.stringify(rows[rows.length - 1]) + '\n');
    console.log(`${ok ? 'OK  ' : 'BAD '} ${stack} ${id} ${j.name ?? 'pass'} -> ${err ? 'ERROR ' + err : (v.pass ? 'PASS' : 'FAIL ' + v.failedIds.join(','))}`);
  }
}
fs.writeFileSync(path.join(EX, '..', 'tmp', 'validation_rows.json'), JSON.stringify({ date: new Date().toISOString(), loadavg_start: os.loadavg(), secs: (Date.now() - t0) / 1000, rows }, null, 1));
console.log('done', rows.filter(r => r.ok).length, '/', rows.length, 'ok');
