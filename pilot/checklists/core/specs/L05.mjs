import { R, ok, bad, cannot, norm, median } from './_common.mjs';

export default {
  id: 'L05', category: '1 Layout-heavy', baseline: 'B1',
  intent: 'Photo-free blog article page: title, author line, 6 paragraphs, a pull quote, comfortable reading width',
  fixture: ['The article has one title (the page\'s top-level heading), an author line, six body paragraphs of two to four sentences each, and one pull quote that is not one of the six paragraphs. Section headings, if any, are exactly one level below the title.'],
  conventions: [
    'body text = visible nodes with ≥ 2 laid-out lines at 1280 px and the most common font size among such nodes (a pull quote set in a different size is therefore excluded automatically)',
  ],
  items: [
    {
      id: 'L05.1', kind: 'A', g2: 'Line length 45–90 characters at desktop',
      op: ['At 1280×800. Body blocks = visible nodes with ≥ 2 lines (`line_chars`) whose font size equals the most common font size among multi-line text nodes; at least 3 such blocks are required.',
        'Take every line except the last of each block (full lines). Pass iff the median of these line lengths is within [45, 90] AND at least 90% of them lie within [40, 95] (±5 characters of tolerance for ragged-right wrapping).'],
      feedback: 'Not met: body text lines must be 45–90 characters long at desktop width.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const multi = s.textNodes().filter(n => n.line_chars.length >= 2);
        if (!multi.length) return cannot('no multi-line text blocks');
        const counts = new Map(); multi.forEach(n => counts.set(n.style.font_size, (counts.get(n.style.font_size) ?? 0) + 1));
        const fs = [...counts.entries()].sort((a, b) => b[1] - a[1])[0][0];
        const body = multi.filter(n => n.style.font_size === fs);
        if (body.length < 3) return cannot(`only ${body.length} multi-line body blocks`);
        const lens = body.flatMap(n => n.line_chars.slice(0, -1));
        const med = median(lens); const inBand = lens.filter(l => l >= 40 && l <= 95).length / lens.length;
        if (med < 45 || med > 90) return bad(`median line length ${med}`);
        if (inBand < 0.9) return bad(`only ${(inBand * 100).toFixed(0)}% of lines within 40–95 chars`);
        return ok();
      },
    },
    { id: 'L05.2', kind: 'H', g2: 'Pull quote visually distinct', op: ['not judged (D-5)'], feedback: null },
    {
      id: 'L05.3', kind: 'A', g2: 'Heading hierarchy correct',
      op: ['Headings = visible nodes with role `heading`, in reading order, at 1280 px. Pass iff: ≥ 1 heading; the first heading has level 1; exactly one level-1 heading; and no heading\'s level exceeds the previous heading\'s level by more than 1.'],
      feedback: 'Not met: the heading hierarchy must be correct (one top-level title first, no skipped levels).',
      async run(ctx) {
        const s = await ctx.at(1280);
        const hs = s.byRole('heading');
        if (!hs.length) return bad('no headings');
        const lv = hs.map(h => h.states.level ?? 2);
        if (lv[0] !== 1) return bad(`first heading is level ${lv[0]}`);
        if (lv.filter(l => l === 1).length !== 1) return bad(`${lv.filter(l => l === 1).length} level-1 headings`);
        for (let i = 1; i < lv.length; i++) if (lv[i] > lv[i - 1] + 1) return bad(`level jumps ${lv[i - 1]}→${lv[i]}`);
        return ok();
      },
    },
  ],
};
