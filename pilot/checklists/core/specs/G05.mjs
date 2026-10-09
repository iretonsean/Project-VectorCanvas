import { R, ok, bad, cannot, norm, btn } from './_common.mjs';

const STEP_RE = /^step\s*(\d+)\b/i;
const NEXT = /^(next|continue|finish)\b/i, BACK = /^(back|previous)\b/i;
async function step(ctx) {
  const s = await ctx.now();
  const hs = s.byRole('heading').filter(h => STEP_RE.test(norm(h.name ?? h.text)));
  if (hs.length !== 1) return { n: null, s, why: `${hs.length} visible headings starting "Step N" (need exactly 1)` };
  return { n: +STEP_RE.exec(norm(hs[0].name ?? hs[0].text))[1], s };
}
async function answer(ctx, yes) {
  const s = await ctx.now();
  const re = yes ? /^yes\b/i : /^no\b/i;
  const c = s.byRole(['radio', 'button', 'checkbox', 'switch'], re)[0];
  if (!c) return 'no Yes/No control on step 2';
  await ctx.activate(c); return null;
}
async function next(ctx) { const s = await ctx.now(); const b = btn(s, NEXT); if (!b) return 'no Next button'; await ctx.activate(b); return null; }
async function back(ctx) { const s = await ctx.now(); const b = btn(s, BACK); if (!b) return 'no Back button'; await ctx.activate(b); return null; }
async function to2(ctx) { await ctx.at(1280); let st = await step(ctx); if (st.n !== 1) return st.why ?? `starts on step ${st.n}`; const e = await next(ctx); if (e) return e; st = await step(ctx); return st.n === 2 ? null : `Next from step 1 shows step ${st.n}`; }

export default {
  id: 'G05', category: '4 Logic-heavy', baseline: 'B2',
  intent: 'Wizard whose step 3 is skipped when the user answers "no" in step 2',
  fixture: ['A four-step wizard. Exactly one heading is visible at a time and it starts with "Step N" (N = 1 to 4), for example "Step 2: Pet". Every step has a "Next" button (on step 4 it may be named "Finish") and every step except the first has a "Back" button. Step 2 asks "Do you have a pet?" with two radio buttons named "Yes" and "No". Step 3 ("Step 3: Pet name") has a text field and is skipped when the answer to step 2 is "No". Step 4 is a summary. "Back" always returns to the step the user actually came from.'],
  conventions: [
    'current step = the number in the single visible heading whose text starts with /^step\\s*\\d+/i',
    'answer controls = radio (or button/checkbox/switch) named /^yes/i or /^no/i; next = button named /^(next|continue|finish)/i; back = button named /^(back|previous)/i',
  ],
  items: [
    {
      id: 'G05.1', kind: 'A', trace: true, g2: 'Skip logic correct both ways',
      op: ['Fresh load at 1280 px starts on step 1. Next → step 2.',
        'Path "No": answer No, Next → step 4 (step 3 skipped). Path "Yes": (new fresh load) answer Yes, Next → step 3, Next → step 4.'],
      feedback: 'Not met: step 3 must be skipped when the answer in step 2 is "No" and shown when it is "Yes".',
      async run(ctx) {
        let e = await to2(ctx); if (e) return cannot(e);
        e = await answer(ctx, false); if (e) return cannot(e);
        e = await next(ctx); if (e) return cannot(e);
        let st = await step(ctx); if (st.n !== 4) return bad(`after answering No, Next shows step ${st.n}`);
        await ctx.reload();
        e = await to2(ctx); if (e) return cannot(e);
        e = await answer(ctx, true); if (e) return cannot(e);
        e = await next(ctx); if (e) return cannot(e);
        st = await step(ctx); if (st.n !== 3) return bad(`after answering Yes, Next shows step ${st.n}`);
        e = await next(ctx); if (e) return cannot(e);
        st = await step(ctx); return st.n === 4 ? ok() : bad(`step 3 → Next shows step ${st.n}`);
      },
    },
    {
      id: 'G05.2', kind: 'A', trace: true, g2: 'Back from step 4 returns to the right step',
      op: ['Path "No" to step 4, press Back → step 2. Fresh load, path "Yes" to step 4, press Back → step 3.'],
      feedback: 'Not met: Back from step 4 must return to the step the user came from (step 2 after "No", step 3 after "Yes").',
      async run(ctx) {
        let e = await to2(ctx); if (e) return cannot(e);
        e = await answer(ctx, false); if (e) return cannot(e);
        e = await next(ctx); if (e) return cannot(e);
        let st = await step(ctx); if (st.n !== 4) return cannot(`path No reaches step ${st.n}, not 4`);
        e = await back(ctx); if (e) return cannot(e);
        st = await step(ctx); if (st.n !== 2) return bad(`Back from step 4 (after No) shows step ${st.n}, expected 2`);
        await ctx.reload();
        e = await to2(ctx); if (e) return cannot(e);
        e = await answer(ctx, true); if (e) return cannot(e);
        e = await next(ctx); if (e) return cannot(e);
        e = await next(ctx); if (e) return cannot(e);
        st = await step(ctx); if (st.n !== 4) return cannot(`path Yes reaches step ${st.n}, not 4`);
        e = await back(ctx); if (e) return cannot(e);
        st = await step(ctx); return st.n === 3 ? ok() : bad(`Back from step 4 (after Yes) shows step ${st.n}, expected 3`);
      },
    },
  ],
};
