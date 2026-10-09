import { R, ok, bad, cannot, norm, btn, inPageDialog } from './_common.mjs';

const PROMPT_RE = /discard|unsaved|lose|changes/i;
const CANCEL_RE = /^(cancel|keep|stay|no|go back|continue editing)/i;

async function setup(ctx) {
  ctx.dialogPolicy = 'dismiss';
  const s = await ctx.at(1280);
  const tabs = s.byRole('tab');
  if (tabs.length < 2) return { err: cannot(`${tabs.length} tabs with role tab (need 2)`) };
  const find = re => tabs.find(t => re.test(t.name ?? ''));
  const profile = find(/profile/i) ?? tabs[0], notes = find(/notes/i) ?? tabs[1];
  return { s, profile, notes };
}
const selectedName = s => s.byRole('tab').find(t => t.states.selected)?.name ?? null;
const field = s => s.byRole(['text-area', 'text-field']).find(n => true) ?? null;
/** was a discard prompt shown after the last action? native confirm (probe.dialogs) or an in-page dialog */
async function prompt(ctx, before) {
  const s = await ctx.now();
  const nat = ctx.dialogs.length > before.dialogs;
  const dlg = inPageDialog(s)[0];
  const txt = s.textNodes().some(n => PROMPT_RE.test(n.text) && !before.texts.has(n.text));
  return { shown: nat || !!dlg || txt, native: nat, dlg, s, nativeMsg: nat ? ctx.dialogs[ctx.dialogs.length - 1].message : null };
}
const snapState = async ctx => { const s = await ctx.now(); return { dialogs: ctx.dialogs.length, texts: new Set(s.textNodes().map(n => n.text)) }; };

export default {
  id: 'G03', category: '4 Logic-heavy', baseline: 'B2',
  intent: 'Tabs with unsaved-changes guard: switching tabs with edits prompts "discard?"',
  fixture: ['Two tabs named "Profile" and "Notes" (tab semantics: a tab list with tabs and tab panels). Each tab has one multi-line text field. Typing in the text field of the current tab makes that tab "dirty". Switching to the other tab while the current tab is dirty asks "Discard changes?" with a "Discard" button and a "Cancel" button (either a browser confirm() dialog or an in-page dialog). Cancel stays on the current tab and keeps the typed text; Discard switches tabs and drops the text. Switching while the tab is not dirty switches immediately with no prompt.'],
  conventions: [
    'tabs = nodes with role `tab` (≥ 2); the selected tab has `states.selected` = true',
    'a prompt = a native confirm dialog (recorded by the browser driver; PVC has none) OR a visible `dialog`/`alert-dialog` node OR newly visible text matching /discard|unsaved|lose|changes/i',
    'cancel (in-page) = button named /^(cancel|keep|stay|no|go back|continue editing)/i; for a native confirm the driver dismisses it',
    'edit field = first visible `text-area` or `text-field`',
  ],
  items: [
    {
      id: 'G03.1', kind: 'A', trace: true, g2: 'Prompt only when dirty',
      op: ['Fresh load. Activate the second tab (no edits yet) → no prompt, and the second tab becomes selected. Activate the first tab again (still clean) → no prompt.',
        'Then type "x" into the edit field of the first tab (dirty) and activate the second tab → a prompt is shown.'],
      feedback: 'Not met: switching tabs must prompt to discard only when the current tab has unsaved edits.',
      async run(ctx) {
        const r = await setup(ctx); if (r.err) return r.err;
        let before = await snapState(ctx);
        await ctx.activate(r.notes);
        let p = await prompt(ctx, before);
        if (p.shown) return bad('prompt shown although nothing was edited');
        if (selectedName(p.s) !== r.notes.name) return bad('clean switch did not select the other tab');
        let s = await ctx.now(); const prof = s.byRole('tab').find(t => t.id === r.profile.id);
        before = await snapState(ctx); await ctx.activate(prof);
        p = await prompt(ctx, before); if (p.shown) return bad('prompt shown on clean switch back');
        s = await ctx.now(); const f = field(s); if (!f) return cannot('no edit field');
        await ctx.type(f, 'x');
        s = await ctx.now(); const notes = s.byRole('tab').find(t => t.id === r.notes.id);
        before = await snapState(ctx); await ctx.activate(notes);
        p = await prompt(ctx, before);
        if (!p.shown) return bad('no prompt when switching away from a dirty tab');
        if (p.native && !PROMPT_RE.test(p.nativeMsg ?? '')) return bad(`prompt text "${p.nativeMsg}" does not ask about discarding`);
        return ok();
      },
    },
    {
      id: 'G03.2', kind: 'A', trace: true, g2: 'Cancel keeps the tab',
      op: ['Fresh load; type "x" into the first tab\'s edit field; activate the second tab → prompt; cancel it (native: dismissed; in-page: press the Cancel button). Afterwards the first tab is still the selected tab and its edit field still contains "x".'],
      feedback: 'Not met: cancelling the discard prompt must keep the current tab and its unsaved text.',
      async run(ctx) {
        const r = await setup(ctx); if (r.err) return r.err;
        let s = await ctx.now(); const f = field(s); if (!f) return cannot('no edit field');
        await ctx.type(f, 'x');
        s = await ctx.now(); const notes = s.byRole('tab').find(t => t.id === r.notes.id);
        const before = await snapState(ctx); await ctx.activate(notes);
        const p = await prompt(ctx, before);
        if (!p.shown) return cannot('no prompt was shown, nothing to cancel');
        if (!p.native) {
          const c = p.s.byRole('button', CANCEL_RE)[0]; if (!c) return cannot('prompt has no Cancel button');
          await ctx.activate(c);
        }
        s = await ctx.now();
        if (selectedName(s) !== r.profile.name) return bad(`after Cancel the selected tab is "${selectedName(s)}"`);
        const f2 = field(s);
        if (!f2 || !String(f2.states.value ?? '').includes('x')) return bad('typed text lost after Cancel');
        return ok();
      },
    },
  ],
};
