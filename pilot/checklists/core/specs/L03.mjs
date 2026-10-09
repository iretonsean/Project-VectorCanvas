import { R, ok, bad, cannot, norm, CTA_ROLES, hasPaint } from './_common.mjs';

/** illustration = largest visible leaf-like visual block: svg/img/canvas, or a painted box with no text and no interactive descendants,
 *  >= 120 px on both sides at 1280 (>= 60 at phones), narrower than 90% of the viewport */
export function findIllustration(s) {
  const cands = s.nodes.filter(n => {
    if (!s.isVisible(n) || n.kind === 'body' || n.kind === 'html') return false;
    const [x, y, w, h] = n.rect;
    if (w < 60 || h < 60) return false;
    if (w >= 0.9 * s.vw && h > 0.9 * s.vh) return false;
    const ds = s.descendants(n);
    if (n.text || ds.some(d => d.text || (d.role && CTA_ROLES.includes(d.role)))) return false;
    if (n.visual) return true;
    return hasPaint(n) && (n.style.bg || n.style.bg_unknown) && !ds.some(d => d.role);
  });
  cands.sort((a, b) => R.area(b.rect) - R.area(a.rect));
  return cands[0] ?? null;
}
export function heroText(s, ill) {
  return s.nodes.filter(n => s.isVisible(n) && ((n.text && !s.contains(ill, n)) || (CTA_ROLES.includes(n.role) && !s.contains(ill, n))));
}

export default {
  id: 'L03', category: '1 Layout-heavy', baseline: 'B1',
  intent: 'Landing hero: big headline, sub-headline, two buttons (primary and secondary), illustration on the right that moves below text on narrow screens',
  fixture: ['No image files or network resources are available: draw the illustration with inline vector shapes or styled boxes (at least 160 px wide on desktop). The hero has one headline, one sub-headline and exactly two buttons.'],
  conventions: [
    'illustration = the largest visible leaf block that has an image/graphic or a solid fill and contains no text or controls',
    'hero text column = every other visible text node and button/link',
  ],
  items: [
    {
      id: 'L03.1', kind: 'A', g2: 'Reflow at the breakpoint',
      op: ['Illustration and text column as in the conventions.',
        'At 1280 and 1024 px: the illustration lies to the RIGHT of the text column (illustration.x ≥ the largest right edge among text/button nodes − 2 px) and overlaps the text column vertically.',
        'At 480 and 320 px: the illustration lies BELOW the text column (illustration.y ≥ the largest bottom edge among text/button nodes − 2 px) and overlaps it horizontally.'],
      feedback: 'Not met: the illustration must sit on the right of the text on wide screens and move below the text on narrow screens.',
      async run(ctx) {
        for (const w of [1280, 1024, 480, 320]) {
          const s = await ctx.at(w);
          const ill = findIllustration(s);
          if (!ill) return cannot(`${w}px: no illustration block found`);
          const tx = heroText(s, ill);
          if (!tx.length) return cannot(`${w}px: no text found`);
          const col = R.union(tx.map(n => n.lines.length ? R.union(n.lines) : n.rect));
          const maxRight = Math.max(...tx.map(n => R.r(n.lines.length ? R.union(n.lines) : n.rect)));
          const maxBottom = Math.max(...tx.map(n => R.b(n.lines.length ? R.union(n.lines) : n.rect)));
          if (w >= 1024) {
            if (ill.rect[0] < maxRight - 2) return bad(`${w}px: illustration not right of text (x=${Math.round(ill.rect[0])}, text right=${Math.round(maxRight)})`);
            if (R.vOverlap(ill.rect, col) <= 0) return bad(`${w}px: illustration not beside the text`);
          } else {
            if (ill.rect[1] < maxBottom - 2) return bad(`${w}px: illustration not below text (y=${Math.round(ill.rect[1])}, text bottom=${Math.round(maxBottom)})`);
            if (R.hOverlap(ill.rect, col) <= 0) return bad(`${w}px: illustration off to the side when stacked`);
          }
        }
        return ok();
      },
    },
    { id: 'L03.2', kind: 'H', g2: 'Primary vs secondary styling distinct', op: ['not judged (D-5)'], feedback: null },
    {
      id: 'L03.3', kind: 'A', g2: 'Headline is a level-1 heading',
      op: ['At 1280 px: exactly one visible node has role `heading` with `states.level` = 1, and its text is the node with the largest font size among all visible text nodes (ties allowed).'],
      feedback: 'Not met: the headline must be the page\'s single level-1 heading.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const h1 = s.byRole('heading').filter(n => n.states.level === 1);
        if (h1.length !== 1) return bad(`found ${h1.length} level-1 headings`);
        const maxFs = Math.max(...s.textNodes().map(n => n.style.font_size));
        const h1fs = Math.max(...[h1[0], ...s.descendants(h1[0])].filter(n => n.text).map(n => n.style.font_size));
        if (h1fs < maxFs - 0.5) return bad(`level-1 heading ${h1fs}px is not the largest text (${maxFs}px)`);
        return ok();
      },
    },
  ],
};
