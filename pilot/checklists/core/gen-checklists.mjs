// core/gen-checklists.mjs — renders checklists/<ID>.md, checklists/U.md and checklists/INDEX.md (with sha256) from the spec modules.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { U } from './universal.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const OUT = path.join(HERE, '..', 'checklists');
const IDS = ['L01', 'L02', 'L03', 'L04', 'L05', 'L06', 'L07', 'L08', 'L09', 'L10', 'G01', 'G02', 'G03', 'G04', 'G05'];
const sha = b => crypto.createHash('sha256').update(b).digest('hex');
fs.mkdirSync(OUT, { recursive: true });

const G2_COMMON = 'Source of truth for the checklist items: `docs/G2_Benchmark_Corpus.md` §5 (main @ 5fdd412). Item order and wording below follow the G2 row. **A** = automatable (decided by the checker from observables), **H** = human-judged: *not judged (D-5)*; pass is decided on A items only.';

function md(spec) {
  const L = [];
  L.push(`# ${spec.id} — checklist specification (stack-neutral)`);
  L.push('');
  L.push(`**Category:** ${spec.category}  |  **Primary baseline (G2):** ${spec.baseline}  |  **Status:** written before any agent output exists.`);
  L.push('');
  L.push(`**G2 intent:** ${spec.intent}`);
  L.push('');
  L.push(G2_COMMON);
  L.push('');
  L.push('## Fixture');
  L.push('');
  L.push('Sentence(s) to append **verbatim** to the intent text, for **both** stacks (needed so the A items can be evaluated; the G2 intent alone does not contain this data):');
  L.push('');
  for (const f of spec.fixture) L.push(`> ${f}`, '');
  L.push('## How the checks observe the app');
  L.push('');
  L.push('All checks are predicates over the stack-neutral **Snapshot** (`PVC_CHECKER_NEEDS.md` §1) and a small event vocabulary (click, type, key, new session). Baseline: Chromium 141, offline, DOM geometry + Chromium accessibility tree. PVC: `pvc_runtime` headless. Geometry widths: 320, 480, 768, 1024, 1280 px; default height 800; text scale 1 unless stated.');
  L.push('');
  L.push('**Locator conventions used by this specimen:**');
  L.push('');
  for (const c of spec.conventions) L.push(`- ${c}`);
  L.push('');
  L.push('## Checklist items');
  L.push('');
  for (const it of spec.items) {
    L.push(`### ${it.id} — ${it.g2}  \`[${it.kind}]\`${it.trace ? '  *(scripted interaction; replayed by keyboard in U2)*' : ''}`);
    L.push('');
    if (it.kind === 'H') { L.push('**H item — not judged (D-5).** Reported as "not judged"; it does not enter the pass decision.', ''); continue; }
    L.push('**Operational definition**');
    L.push('');
    it.op.forEach(o => L.push(`- ${o}`));
    L.push('');
    L.push(`**Standardised failure text (the only repair feedback for this item, G2 §4.4):** \`${it.feedback}\``);
    L.push('');
  }
  L.push('## Universal items');
  L.push('');
  L.push('U1–U5 apply to this specimen unchanged: see [`U.md`](U.md).');
  L.push('');
  L.push('## Pass decision');
  L.push('');
  const a = spec.items.filter(i => i.kind === 'A').length, h = spec.items.filter(i => i.kind === 'H').length;
  L.push(`PASS ⇔ all ${a} specimen A item(s) **and** U1–U5 pass (${a + 5} A items in total; ${h} H item(s) not judged). Build failure (baseline: compile/bundle/type-check; PVC: validator error) = FAIL.`);
  L.push('');
  return L.join('\n');
}

function uMd() {
  const L = ['# U1–U5 — universal checklist (stack-neutral)', '', G2_COMMON, '',
    'Applies to every specimen in both stacks (G2 §3.1). Widths tested: 320, 480, 768, 1024, 1280 px (height 800) unless stated; text scale 1 unless stated.', ''];
  for (const it of U) {
    L.push(`## ${it.id} — ${it.g2}  \`[A]\``, '', '**Operational definition**', '');
    it.op.forEach(o => L.push(`- ${o}`));
    L.push('', `**Standardised failure text:** \`${it.feedback}\``, '');
  }
  return L.join('\n');
}

const files = [];
for (const id of IDS) {
  const spec = (await import(`./specs/${id}.mjs`)).default;
  const text = md(spec);
  fs.writeFileSync(path.join(OUT, `${id}.md`), text);
  files.push([`checklists/${id}.md`, sha(text)]);
  files.push([`core/specs/${id}.mjs`, sha(fs.readFileSync(path.join(HERE, 'specs', `${id}.mjs`)))]);
}
const u = uMd(); fs.writeFileSync(path.join(OUT, 'U.md'), u);
files.push(['checklists/U.md', sha(u)]);
for (const f of ['core/lib.mjs', 'core/ctx.mjs', 'core/universal.mjs', 'core/runner.mjs', 'core/specs/_common.mjs'])
  files.push([f, sha(fs.readFileSync(path.join(HERE, '..', f)))]);
files.push(['PVC_CHECKER_NEEDS.md', sha(fs.readFileSync(path.join(HERE, '..', 'PVC_CHECKER_NEEDS.md')))]);

const I = ['# Checklist INDEX', '',
  'Frozen checklist specifications for the G2 pilot (B4): 15 specimens (L01–L10, G01–G05) + universal items U1–U5.',
  'The `.md` files are generated from the executable spec modules (`core/specs/*.mjs`) so the prose and the code cannot diverge; both are hashed. sha256 over file bytes. Regenerate with `node core/gen-checklists.mjs`.',
  '', '| file | sha256 |', '|---|---|', ...files.map(([f, h]) => `| \`${f}\` | \`${h}\` |`), ''];
fs.writeFileSync(path.join(OUT, 'INDEX.md'), I.join('\n'));
console.log('wrote', files.length, 'hashes');
