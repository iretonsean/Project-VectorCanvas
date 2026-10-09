import { R, ok, bad, cannot, norm } from './_common.mjs';
import { CONTROL_ROLES } from '../lib.mjs';

/** rows: for each visible form control, its visible label node = visible text node whose own text equals the control's accessible name */
export function settingsRows(s) {
  const controls = s.byRole([...CONTROL_ROLES]).filter(n => !n.a11y_hidden || true);
  const rows = [];
  for (const c of controls) {
    const nm = norm(c.name);
    const cand = nm ? s.nodes.filter(n => s.isVisible(n) && n.text && norm(n.text) === nm && !s.contains(c, n)) : [];
    // nearest label by vertical overlap, then closest
    cand.sort((a, b) => Math.abs(R.cy(a.rect) - R.cy(c.rect)) - Math.abs(R.cy(b.rect) - R.cy(c.rect)));
    rows.push({ control: c, label: cand[0] ?? null });
  }
  return rows;
}

export default {
  id: 'L06', category: '1 Layout-heavy', baseline: 'B1',
  intent: 'Settings page with grouped sections, each with a title and rows of label + control on the right',
  fixture: ['Three groups (for example Account, Notifications, Appearance), each with a heading and at least two rows. Each row has a visible text label on the left and a form control (switch, checkbox, select or text field) on the right whose accessible name is that label text.'],
  conventions: [
    'controls = visible checkbox, switch, radio, slider, spinbutton, text inputs, combo box, list box',
    'the row label = the visible text node whose own text equals the control\'s accessible name',
    'groups = headings other than the level-1 page title (the level-1 title is ignored when other headings exist)',
  ],
  items: [
    {
      id: 'L06.1', kind: 'A', g2: 'Labels aligned in one column',
      op: ['At 1280 px: ≥ 4 controls, each with a located visible label (conventions) on the same row (vertical overlap) and to the left of its control. Pass iff all labels\' left edges are equal within 2 px.'],
      feedback: 'Not met: all row labels must be aligned in one column.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const rows = settingsRows(s);
        if (rows.length < 4) return cannot(`only ${rows.length} controls`);
        const miss = rows.filter(r => !r.label); if (miss.length) return bad(`${miss.length} controls without a visible label equal to their name`);
        for (const r of rows) if (!(R.leftOf(r.label.rect, r.control.rect, 2) && R.vOverlap(r.label.rect, r.control.rect) > 0)) return bad(`label "${norm(r.label.text)}" is not left of its control on one row`);
        const xs = rows.map(r => r.label.rect[0]);
        if (Math.max(...xs) - Math.min(...xs) > 2) return bad(`label left edges vary by ${(Math.max(...xs) - Math.min(...xs)).toFixed(1)}px`);
        return ok();
      },
    },
    {
      id: 'L06.2', kind: 'A', g2: 'Controls right-aligned',
      op: ['At 1280 px, same rows as L06.1. Pass iff all control right edges are equal within 2 px.'],
      feedback: 'Not met: all row controls must be right-aligned.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const rows = settingsRows(s);
        if (rows.length < 4) return cannot(`only ${rows.length} controls`);
        const xs = rows.map(r => R.r(r.control.rect));
        if (Math.max(...xs) - Math.min(...xs) > 2) return bad(`control right edges vary by ${(Math.max(...xs) - Math.min(...xs)).toFixed(1)}px`);
        return ok();
      },
    },
    {
      id: 'L06.3', kind: 'A', g2: 'Groups have headings',
      op: ['At 1280 px: group headings = visible `heading` nodes excluding level 1 (if only level-1 headings exist, they count). ≥ 2 group headings. In reading order, every control comes after some group heading, and each group heading is followed (before the next heading) by ≥ 2 controls.'],
      feedback: 'Not met: every group of settings must have a heading.',
      async run(ctx) {
        const s = await ctx.at(1280);
        let hs = s.byRole('heading'); const non1 = hs.filter(h => h.states.level !== 1); if (non1.length) hs = non1;
        if (hs.length < 2) return bad(`${hs.length} group headings`);
        const ctrls = s.byRole([...CONTROL_ROLES]);
        const evs = [...hs.map(h => ({ t: 'h', n: h })), ...ctrls.map(c => ({ t: 'c', n: c }))].sort((a, b) => s.index.get(a.n.id) - s.index.get(b.n.id));
        let cur = null, counts = [];
        for (const e of evs) { if (e.t === 'h') { cur = { h: e.n, c: 0 }; counts.push(cur); } else { if (!cur) return bad('a control precedes every group heading'); cur.c++; } }
        const small = counts.filter(c => c.c < 2); if (small.length) return bad(`group "${norm(small[0].h.text || small[0].h.name)}" has ${small[0].c} controls`);
        return ok();
      },
    },
  ],
};
