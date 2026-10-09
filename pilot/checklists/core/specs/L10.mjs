import { R, ok, bad, cannot, norm, isRow, isColumn, shapeOf, iconsIn, iconOk } from './_common.mjs';
import { largestExclusive } from '../lib.mjs';

/** 6 feature cells: headings (level>=2) of the level that has exactly 6; cell = largest ancestor that holds no other title */
export function findCells(s) {
  const hs = s.byRole('heading').filter(h => (h.states.level ?? 2) >= 2);
  const byLevel = new Map(); hs.forEach(h => byLevel.set(h.states.level ?? 2, [...(byLevel.get(h.states.level ?? 2) ?? []), h]));
  const six = [...byLevel.values()].find(a => a.length === 6);
  if (!six) return null;
  return six.map(t => ({ title: t, cell: largestExclusive(s, t, six.filter(o => o.id !== t.id)) }));
}

export default {
  id: 'L10', category: '1 Layout-heavy', baseline: 'B1',
  intent: 'Image-free feature grid: 6 features with icon, title and text in a 3×2 grid, 1 column on phones',
  fixture: ['Each of the six features has an icon (an inline vector icon or an emoji; no image files), a short title that is a heading, and one sentence of text. A section heading above the grid, if present, is a different heading level from the feature titles.'],
  conventions: [
    'feature titles = visible headings of level ≥ 2 at the one heading level that has exactly six of them',
    'cell = the largest ancestor of a title that contains no other feature title',
  ],
  items: [
    {
      id: 'L10.1', kind: 'A', g2: 'Grid 3×2 → 1×6',
      op: ['Six feature cells found. At 1280 and 1024 px the cells form rows [3,3] (3 columns, 2 rows). At 480 and 320 px the cells form one column of 6.'],
      feedback: 'Not met: the six features must be a 3×2 grid on wide screens and a single column on phones.',
      async run(ctx) {
        for (const w of [1280, 1024, 480, 320]) {
          const s = await ctx.at(w); const c = findCells(s);
          if (!c) return cannot(`${w}px: no heading level with exactly six feature titles`);
          const rects = c.map(x => x.cell.rect); const sh = JSON.stringify(shapeOf(rects));
          if (w >= 1024 && sh !== '[3,3]') return bad(`${w}px: grid shape ${sh}`);
          if (w <= 480 && (!isColumn(rects) || sh !== '[1,1,1,1,1,1]')) return bad(`${w}px: grid shape ${sh}`);
        }
        return ok();
      },
    },
    {
      id: 'L10.2', kind: 'A', g2: 'Equal gaps',
      op: ['At 1280 px: the 4 horizontal gaps between neighbouring cells in each row and the 3 vertical gaps between the two rows are all equal within 2 px. At 480 px the 5 vertical gaps between the stacked cells are equal within 2 px. (Gaps are measured between cell rects; shapes must be as in L10.1.)'],
      feedback: 'Not met: the gaps between the feature cells must all be equal.',
      async run(ctx) {
        let s = await ctx.at(1280); let c = findCells(s);
        if (!c) return cannot('no six feature cells');
        let rows = []; { const rs = c.map(x => x.cell.rect); const sorted = [...rs].sort((a, b) => a[1] - b[1] || a[0] - b[0]); rows = [sorted.slice(0, 3).sort((a, b) => a[0] - b[0]), sorted.slice(3).sort((a, b) => a[0] - b[0])]; }
        if (rows[0].length !== 3 || rows[1].length !== 3 || JSON.stringify(shapeOf(c.map(x => x.cell.rect))) !== '[3,3]') return cannot('not a 3×2 grid at 1280');
        const gaps = [];
        for (const r of rows) for (let i = 0; i < 2; i++) gaps.push(r[i + 1][0] - R.r(r[i]));
        for (let i = 0; i < 3; i++) gaps.push(rows[1][i][1] - R.b(rows[0][i]));
        if (Math.max(...gaps) - Math.min(...gaps) > 2) return bad(`gaps at 1280px: ${gaps.map(g => Math.round(g)).join(',')}`);
        s = await ctx.at(480); c = findCells(s);
        if (!c) return cannot('480px: no six feature cells');
        const st = c.map(x => x.cell.rect).sort((a, b) => a[1] - b[1]); const g2 = [];
        for (let i = 0; i < 5; i++) g2.push(st[i + 1][1] - R.b(st[i]));
        if (Math.max(...g2) - Math.min(...g2) > 2) return bad(`gaps at 480px: ${g2.map(g => Math.round(g)).join(',')}`);
        return ok();
      },
    },
    {
      id: 'L10.3', kind: 'A', g2: 'Icons decorative or named',
      op: ['At 1280 px: every one of the six cells contains ≥ 1 icon (visible svg/img node or emoji-only text node), and every icon is decorative or named (hidden from the accessibility tree, or an `image` with a name).'],
      feedback: 'Not met: every feature must have an icon that is either decorative (hidden from assistive technology) or named.',
      async run(ctx) {
        const s = await ctx.at(1280); const c = findCells(s);
        if (!c) return cannot('no six feature cells');
        for (const [i, x] of c.entries()) {
          const ic = iconsIn(s, x.cell);
          if (!ic.length) return bad(`feature ${i + 1} has no icon`);
          if (ic.some(k => !iconOk(s, k))) return bad(`feature ${i + 1} has an icon exposed without a name`);
        }
        return ok();
      },
    },
  ],
};
