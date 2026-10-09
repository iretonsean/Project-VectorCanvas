// baseline/tools.mjs — tool outputs for the baseline agent (G2 §4.3 tool parity): html-validate, tsc (B2), axe-core.
// Each returns compact plain text; documented in ../TOOLS.md.
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { HtmlValidate } from 'html-validate';
import { build, serve } from './check.mjs';
import { launchBrowser } from './probe-browser.mjs';
import { cleanup } from './build.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const CAP = 15;
const hv = new HtmlValidate({
  extends: ['html-validate:recommended'],
  rules: { 'no-inline-style': 'off', 'require-sri': 'off', 'no-trailing-whitespace': 'off', 'void-style': 'off', 'attribute-boolean-style': 'off', 'attribute-empty-style': 'off', 'prefer-native-element': 'off', 'no-implicit-close': 'off', 'tel-non-breaking': 'off', 'doctype-style': 'off', 'long-title': 'off' },
  elements: ['html5'],
});

function cap(lines, more) { return lines.length > CAP ? [...lines.slice(0, CAP), `... ${lines.length - CAP} more`] : lines; }

export async function toolHtmlValidate(stack, file) {
  let html;
  if (stack === 'B1') html = fs.readFileSync(file, 'utf8');
  else {
    const b = await build('B2', file);
    if (!b.bundled) { return 'html-validate: cannot run, the program does not bundle'; }
    const srv = await serve(b.dir); const br = await launchBrowser(srv.origin);
    try {
      const p = await br.newProbe();
      html = '<!doctype html>\n' + await p.page.evaluate(() => {
        const c = document.documentElement.cloneNode(true);
        c.querySelectorAll('script,style,link').forEach(e => e.remove());
        c.querySelectorAll('[data-pvc-id]').forEach(e => e.removeAttribute('data-pvc-id'));
        return c.outerHTML.replace(/></g, '>\n<');
      });
    } finally { await br.close(); srv.close(); cleanup(b); }
  }
  const rep = await hv.validateString(html);
  const lines = [];
  for (const r of rep.results) for (const m of r.messages) lines.push(`${m.line}:${m.column} ${m.severity === 2 ? 'error' : 'warn'} ${m.ruleId}: ${m.message.replace(/\s+/g, ' ').slice(0, 160)}`);
  return `html-validate${stack === 'B2' ? ' (rendered DOM)' : ''}: ${lines.length} problem(s)\n` + cap(lines).join('\n');
}

export async function toolTsc(stack, file) {
  if (stack !== 'B2') return 'tsc: not applicable to B1';
  const b = await build('B2', file);
  try {
    const lines = b.tsc.map(t => `${t.file}:${t.line}:${t.col} ${t.code} ${t.msg}`);
    const extra = b.errors.map(e => 'build: ' + e.slice(0, 200));
    return `tsc: ${b.tsc.length} error(s)\n` + cap([...lines, ...extra]).join('\n');
  } finally { cleanup(b); }
}

export async function toolAxe(stack, file) {
  const b = await build(stack, file);
  if (stack === 'B2' && !b.bundled) { cleanup(b); return 'axe: cannot run, the program does not bundle'; }
  const srv = await serve(b.dir); const br = await launchBrowser(srv.origin);
  try {
    const p = await br.newProbe();
    await p.page.addScriptTag({ path: require.resolve('axe-core/axe.min.js') });
    const res = await p.page.evaluate(async () => {
      const r = await window.axe.run(document, { resultTypes: ['violations'] });
      return r.violations.map(v => ({ id: v.id, impact: v.impact, help: v.help, n: v.nodes.length, ex: v.nodes[0].html.replace(/\s+/g, ' ').slice(0, 90) }));
    });
    const order = { critical: 0, serious: 1, moderate: 2, minor: 3 };
    res.sort((a, b) => order[a.impact] - order[b.impact] || a.id.localeCompare(b.id));
    const lines = res.map(v => `${v.impact} ${v.id}: ${v.help} | ${v.n} node(s) | e.g. ${v.ex}`);
    return `axe-core ${require('axe-core/package.json').version}: ${res.length} violation(s)\n` + cap(lines).join('\n');
  } finally { await br.close(); srv.close(); cleanup(b); }
}

export const TOOLS = { 'html-validate': toolHtmlValidate, tsc: toolTsc, axe: toolAxe };
