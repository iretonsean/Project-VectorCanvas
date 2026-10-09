import { R, ok, bad, cannot, norm, isRow, isColumn, shapeOf, isPriceNode, CTA_ROLES } from './_common.mjs';

/** the three plan containers: for each price node, the lowest ancestor that contains >=1 CTA (button/link); distinct. */
export function findPlans(s) {
  const prices = s.nodes.filter(n => s.isVisible(n) && isPriceNode(n));
  const ctas = s.byRole(CTA_ROLES);
  const plans = new Map();
  for (const p of prices) {
    let c = s.parent(p);
    while (c && !ctas.some(x => s.contains(c, x))) c = s.parent(c);
    if (c && c.kind !== 'body') plans.set(c.id, c);
  }
  // drop containers that contain another plan container (page-level wrappers)
  const arr = [...plans.values()];
  return arr.filter(a => !arr.some(b => b.id !== a.id && s.contains(a, b)));
}

export default {
  id: 'L02', category: '1 Layout-heavy', baseline: 'B1',
  intent: 'Pricing page with three plans side by side; the middle one highlighted as "most popular"; stacks vertically on phones',
  fixture: ['Each plan shows a price (for example "$0", "$12", "$29"), a list of exactly four features, and one call-to-action button. Plans are Free, Pro and Team in that order; no external images or fonts are available.'],
  conventions: [
    'price = visible text node of ≤ 30 characters starting with a currency symbol + digit, a number + currency symbol, or the word "Free"',
    'plan container = for each price node, the lowest ancestor that contains at least one button/link; duplicates merged; page-level wrappers that contain another plan container are dropped',
    'features = nodes with role `list-item` inside the plan container',
  ],
  items: [
    {
      id: 'L02.1', kind: 'A', g2: '3 columns ≥ 1024 px, 1 column ≤ 480 px',
      op: ['Plan containers found as in the conventions; exactly 3 required.',
        'At 1024 and 1280 px: the 3 plan rects form one row (x-disjoint within 2 px, vertically overlapping ≥ 50%).',
        'At 480 and 320 px: the 3 plan rects form one column (y-disjoint within 2 px, horizontally overlapping ≥ 60% of the narrower).'],
      feedback: 'Not met: the three plans must be three columns at 1024 px and wider, and a single column at 480 px and narrower.',
      async run(ctx) {
        for (const w of [1280, 1024, 480, 320]) {
          const s = await ctx.at(w);
          const plans = findPlans(s);
          if (plans.length !== 3) return cannot(`${w}px: found ${plans.length} plan containers (need 3)`);
          const rects = plans.map(p => p.rect);
          if (w >= 1024 && !isRow(rects)) return bad(`${w}px: plans not in one row (shape ${JSON.stringify(shapeOf(rects))})`);
          if (w <= 480 && !isColumn(rects)) return bad(`${w}px: plans not in one column (shape ${JSON.stringify(shapeOf(rects))})`);
        }
        return ok();
      },
    },
    { id: 'L02.2', kind: 'H', g2: 'Middle plan visually distinct', op: ['not judged (D-5)'], feedback: null },
    {
      id: 'L02.3', kind: 'A', g2: 'Each plan has price, 4 features, CTA',
      op: ['At 1280 px, for each of the 3 plan containers: ≥ 1 price node inside; exactly 4 visible `list-item` descendants; ≥ 1 visible `button` or `link` inside.'],
      feedback: 'Not met: each plan must have a price, exactly four features (as list items) and a call-to-action button.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const plans = findPlans(s);
        if (plans.length !== 3) return cannot(`found ${plans.length} plan containers (need 3)`);
        for (const [i, p] of plans.entries()) {
          const ds = s.descendants(p).concat([p]);
          const price = ds.filter(d => s.isVisible(d) && isPriceNode(d)).length;
          const feats = ds.filter(d => d.role === 'list-item' && s.isVisible(d)).length;
          const cta = ds.filter(d => CTA_ROLES.includes(d.role) && s.isVisible(d)).length;
          if (!price || feats !== 4 || !cta) return bad(`plan ${i + 1}: price=${price} features=${feats} cta=${cta}`);
        }
        return ok();
      },
    },
  ],
};
