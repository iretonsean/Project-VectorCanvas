import { R, ok, bad, cannot, norm, hasPaint, isRow, shapeOf } from './_common.mjs';

const NUM_RE = /^[\s$€£]*\d[\d,.]*\s?[%kKmMbB]?$/;
const isNum = n => n.text && n.text.length <= 12 && NUM_RE.test(n.text);

export function findTiles(s) {
  const nums = s.nodes.filter(n => s.isVisible(n) && isNum(n));
  const tiles = new Map();
  for (const v of nums) {
    const t = s.closest(s.parent(v) ?? v, n => hasPaint(n) && n.kind !== 'body' && n.kind !== 'html');
    if (!t) continue;
    if (nums.filter(o => s.contains(t, o)).length === 1) tiles.set(t.id, t);
  }
  const arr = [...tiles.values()];
  return arr.filter(a => !arr.some(b => b.id !== a.id && s.contains(a, b)));
}
export function findPanels(s, tiles) {
  const tb = Math.max(...tiles.map(t => R.b(t.rect)));
  const c = s.nodes.filter(n => s.isVisible(n) && hasPaint(n) && n.kind !== 'body' && n.kind !== 'html' && n.rect[1] >= tb - 2 && R.area(n.rect) >= 20000 && s.descendants(n).some(d => d.text));
  return c.filter(a => !c.some(b => b.id !== a.id && s.contains(b, a)));
}

export default {
  id: 'L07', category: '1 Layout-heavy', baseline: 'B1',
  intent: 'Dashboard shell: top bar with search and avatar, 4 KPI tiles in a row, two panels below',
  fixture: ['The top bar has a search field whose accessible name contains "Search" and an avatar. Each of the four KPI tiles is a visually distinct box (fill, border or shadow) showing a label and one number (for example "1,284", "92%", "$4.2k", "37"). The two panels below the tiles are visually distinct boxes with a title. No external resources are available.'],
  conventions: [
    'tile value = visible text of ≤ 12 characters that is a number (optional currency prefix, optional %/k/m suffix)',
    'tile = nearest painted ancestor (fill, border or shadow) of a value that contains exactly one value; outermost only',
    'panels = outermost painted boxes that start at or below the tiles, area ≥ 20 000 px² and contain text',
  ],
  items: [
    {
      id: 'L07.1', kind: 'A', g2: '4 tiles in one row ≥ 1280 px and 2×2 ≤ 768 px',
      op: ['Exactly 4 tiles found (conventions) at every width tested.',
        'At 1280 px the 4 tile rects form one row (x-disjoint within 2 px, vertically overlapping). At 768, 480 and 320 px the tiles form two rows of two (row shape [2,2]).'],
      feedback: 'Not met: the four KPI tiles must be one row at 1280 px and wider and a 2×2 grid at 768 px and narrower.',
      async run(ctx) {
        for (const w of [1280, 768, 480, 320]) {
          const s = await ctx.at(w);
          const tiles = findTiles(s);
          if (tiles.length !== 4) return cannot(`${w}px: found ${tiles.length} tiles (need 4)`);
          const rects = tiles.map(t => t.rect);
          if (w === 1280 && !isRow(rects)) return bad(`1280px: tiles not in one row (shape ${JSON.stringify(shapeOf(rects))})`);
          if (w <= 768 && JSON.stringify(shapeOf(rects)) !== '[2,2]') return bad(`${w}px: tile grid ${JSON.stringify(shapeOf(rects))}, need [2,2]`);
        }
        return ok();
      },
    },
    {
      id: 'L07.2', kind: 'A', g2: 'Search reachable by keyboard',
      op: ['At 1280 px a visible text-input node (`search-field`, `text-field` or `email-field`) with a name matching /search/i exists. Using only Tab presses from the document start it receives focus; typing "abc" with the keyboard makes its `states.value` equal "abc".'],
      feedback: 'Not met: the search field must be reachable and usable by keyboard.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const f = s.byRole(['search-field', 'text-field', 'email-field'], /search/i)[0];
        if (!f) return cannot('no search field');
        const save = ctx.mode; ctx.mode = 'keyboard';
        try { await ctx.kbFocus(f); } catch (e) { ctx.mode = save; return bad('search field not reachable by Tab'); }
        await ctx.p.typeFocused('abc'); ctx.mode = save;
        const s2 = await ctx.now(); const f2 = s2.get(f.id);
        if (!f2 || f2.states.value !== 'abc') return bad(`value after typing: ${JSON.stringify(f2?.states.value)}`);
        return ok();
      },
    },
    {
      id: 'L07.3', kind: 'A', g2: 'Panels equal height',
      op: ['At 1280 px exactly 2 panels found (conventions) and their heights differ by ≤ 2 px.'],
      feedback: 'Not met: the two panels must have equal height.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const tiles = findTiles(s); if (tiles.length !== 4) return cannot(`found ${tiles.length} tiles`);
        const ps = findPanels(s, tiles); if (ps.length !== 2) return cannot(`found ${ps.length} panels (need 2)`);
        const d = Math.abs(ps[0].rect[3] - ps[1].rect[3]);
        return d <= 2 ? ok() : bad(`panel heights ${Math.round(ps[0].rect[3])} vs ${Math.round(ps[1].rect[3])}`);
      },
    },
  ],
};
