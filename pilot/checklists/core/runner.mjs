// core/runner.mjs — runs one specimen spec + the universal items against a driver (browser or PVC).
import { Ctx } from './ctx.mjs';
import { U } from './universal.mjs';

export async function loadSpec(id) {
  const m = await import(new URL(`./specs/${id}.mjs`, import.meta.url));
  return m.default;
}

/** returns {results:[{id,kind,pass,detail,feedback,judged}], log:[]} */
export async function runSpec(spec, driver, { only = null } = {}) {
  const results = []; const log = [];
  const pointerResults = {};
  const items = [...spec.items, ...U];
  for (const it of items) {
    if (only && !only.includes(it.id)) continue;
    if (it.kind === 'H') { results.push({ id: it.id, kind: 'H', pass: null, judged: false, detail: 'not judged (D-5)', feedback: null }); continue; }
    const ctx = new Ctx(driver, 'pointer', pointerResults); ctx.spec = spec;
    let r;
    try {
      await ctx.init();
      r = await it.run(ctx);
    } catch (e) {
      r = { pass: false, detail: 'error: ' + String(e.message || e).split('\n')[0].slice(0, 200), error: true };
    } finally {
      log.push(...ctx.log);
      await ctx.close();
    }
    pointerResults[it.id] = r.pass === true;
    results.push({ id: it.id, kind: 'A', pass: !!r.pass, detail: r.detail || '', feedback: it.feedback, unevaluable: !!r.unevaluable, error: !!r.error, judged: true });
  }
  return { results, log };
}
