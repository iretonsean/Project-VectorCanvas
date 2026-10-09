import { R, ok, bad, cannot, norm, btn } from './_common.mjs';

const HINT = /no notes yet/i;
const hintVisible = s => s.textNodes().some(n => HINT.test(n.text));
const ADD = /^add/i, REMOVE = /remove|delete/i, DISMISS = /dismiss|close|got it|hide/i;

async function load(ctx) {
  const s = await ctx.at(1280);
  return s;
}
async function addNote(ctx) { const s = await ctx.now(); const b = btn(s, ADD); if (!b) throw new Error('no "Add note" button'); await ctx.activate(b); }
async function removeAll(ctx) {
  for (let i = 0; i < 12; i++) { const s = await ctx.now(); const b = s.byRole('button', REMOVE)[0]; if (!b) return; await ctx.activate(b); }
}

export default {
  id: 'G01', category: '4 Logic-heavy', baseline: 'B2',
  intent: 'Empty-state hint shown when a list is empty unless the user dismissed it this session (A§2.9)',
  fixture: ['The page shows a list of notes that starts empty. An "Add note" button appends one note per press; each note has a "Remove" button. While the list is empty, a hint "No notes yet. Add your first note." is shown together with a "Dismiss" button. Pressing "Dismiss" hides the hint for the rest of the session, even if the list later becomes empty again. Do not store the dismissal in persistent storage (localStorage, cookies): when the app is opened again in a new session (a new tab or window; a restart for the PVC runtime) the hint is shown again if the list is empty.'],
  conventions: [
    'hint = visible text matching /no notes yet/i',
    'controls: add = button named /^add/i; remove = buttons named /remove|delete/i; dismiss = button named /dismiss|close|got it|hide/i',
    'new session: baseline = a new page (tab) in the same browser context (fresh sessionStorage/in-memory state, shared localStorage); PVC = `new_session` (in-memory state discarded, persisted app storage kept)',
  ],
  items: [
    {
      id: 'G01.1', kind: 'A', trace: true, g2: 'Hint shown when empty and not dismissed',
      op: ['Fresh load (1280 px): the list is empty (no Remove button) and the hint is visible.',
        'Press Add → the hint is not visible. Press Remove until no Remove button exists → the hint is visible again.'],
      feedback: 'Not met: the empty-state hint must be shown when the list is empty (and not dismissed), and hidden when the list has items.',
      async run(ctx) {
        let s = await load(ctx);
        if (s.byRole('button', REMOVE).length) return bad('list is not empty at start');
        if (!hintVisible(s)) return bad('hint not visible when the list is empty');
        await addNote(ctx); s = await ctx.now();
        if (!s.byRole('button', REMOVE).length) return cannot('Add did not create a removable note');
        if (hintVisible(s)) return bad('hint still visible with a non-empty list');
        await removeAll(ctx); s = await ctx.now();
        if (s.byRole('button', REMOVE).length) return cannot('could not empty the list');
        return hintVisible(s) ? ok() : bad('hint not shown after the list became empty again');
      },
    },
    {
      id: 'G01.2', kind: 'A', trace: true, g2: 'Never shown after dismissal in the session',
      op: ['Fresh load: hint visible. Press the Dismiss button → hint not visible.',
        'Press Add (list non-empty, hint not visible), press Add again, then Remove until the list is empty → the hint is still not visible. Then Add once more → still not visible.'],
      feedback: 'Not met: after the user dismisses the hint it must never be shown again in the same session.',
      async run(ctx) {
        let s = await load(ctx);
        if (!hintVisible(s)) return cannot('hint not visible at start');
        const d = btn(s, DISMISS); if (!d) return cannot('no Dismiss button');
        await ctx.activate(d); s = await ctx.now();
        if (hintVisible(s)) return bad('hint still visible after Dismiss');
        await addNote(ctx); await addNote(ctx); s = await ctx.now();
        if (hintVisible(s)) return bad('hint visible after dismissal when the list was non-empty');
        await removeAll(ctx); s = await ctx.now();
        if (s.byRole('button', REMOVE).length) return cannot('could not empty the list');
        if (hintVisible(s)) return bad('hint came back in the same session after the list became empty again');
        return ok();
      },
    },
    {
      id: 'G01.3', kind: 'A', trace: true, g2: 'Returns in a new session',
      op: ['Fresh load: Dismiss the hint (hint hidden). Start a new session (conventions). The list is empty and the hint is visible again.'],
      feedback: 'Not met: the hint must return in a new session.',
      async run(ctx) {
        let s = await load(ctx);
        const d = btn(s, DISMISS); if (!d) return cannot('no Dismiss button');
        await ctx.activate(d); s = await ctx.now();
        if (hintVisible(s)) return bad('hint still visible after Dismiss');
        await ctx.newSession(); s = await ctx.at(1280);
        if (s.byRole('button', REMOVE).length) return bad('list not empty in a new session');
        return hintVisible(s) ? ok() : bad('hint not shown in a new session');
      },
    },
  ],
};
