import { R, ok, bad, cannot, norm, hasPaint } from './_common.mjs';

const S = { initials: 'JD', name: 'Jane Doe', title: 'Product Designer', bio: 'Building the future of spatial computing via GPU pipelines.', button: 'Connect' };
const first = (s, str) => s.byText(str)[0] ?? null;

export default {
  id: 'L01', category: '1 Layout-heavy', baseline: 'B1',
  intent: 'Profile card: avatar with initials, name, job title, one-sentence bio, full-width "Connect" button (the README card)',
  fixture: [
    'None beyond the README English request, which already contains all five strings. If the intent is paraphrased, these strings must stay verbatim: initials "JD", name "Jane Doe", job title "Product Designer", bio "Building the future of spatial computing via GPU pipelines.", button label "Connect".',
  ],
  conventions: ['Strings are looked up as the exact own-text of a visible node (case-sensitive).'],
  items: [
    {
      id: 'L01.1', kind: 'A', g2: 'Name bold and larger than title',
      op: ['Snapshot at 1280 px. name = visible node whose own text is exactly "Jane Doe"; title = visible node whose own text is exactly "Product Designer".',
        'Pass iff name.style.font_weight ≥ 600 AND name.style.font_size > title.style.font_size.'],
      feedback: 'Not met: the name must be bold and larger than the job title.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const n = first(s, S.name), t = first(s, S.title);
        if (!n || !t) return cannot('name or title text not found');
        if (n.style.font_weight < 600) return bad(`name weight ${n.style.font_weight}`);
        if (!(n.style.font_size > t.style.font_size)) return bad(`name ${n.style.font_size}px not larger than title ${t.style.font_size}px`);
        return ok();
      },
    },
    {
      id: 'L01.2', kind: 'A', g2: 'Avatar circular, left of name',
      op: ['Snapshot at 1280 px. initials node = visible node with own text "JD"; avatar = its nearest ancestor-or-self that has an opaque own fill (`style.bg` alpha ≥ 0.5) and a roughly square box (|w−h| ≤ 2 px, 16 ≤ w ≤ 160).',
        'Circular: avatar.style.radius ≥ min(w,h)/2 − 1 (corner radius is clamped to half the shorter side). Left of name: avatar right edge ≤ name left edge + 2 px, and the avatar\'s vertical extent overlaps the name text box.'],
      feedback: 'Not met: the avatar must be circular and placed to the left of the name.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const ini = first(s, S.initials), name = first(s, S.name);
        if (!ini || !name) return cannot('initials or name text not found');
        const av = s.closest(ini, n => n.style.bg && n.style.bg[3] >= 0.5 && Math.abs(n.rect[2] - n.rect[3]) <= 2 && n.rect[2] >= 16 && n.rect[2] <= 160);
        if (!av) return bad('no square filled avatar box around the initials');
        const [x, y, w, h] = av.rect;
        if (av.style.radius < Math.min(w, h) / 2 - 1) return bad(`avatar radius ${av.style.radius} < ${Math.min(w, h) / 2 - 1}`);
        if (!R.leftOf(av.rect, name.rect, 2)) return bad('avatar is not left of the name');
        if (R.vOverlap(av.rect, name.rect) <= 0) return bad('avatar and name not on the same row');
        return ok();
      },
    },
    {
      id: 'L01.3', kind: 'A', g2: 'Button spans card width',
      op: ['Snapshot at 1280 px. button = visible node with role `button` whose name matches /connect/i. card = nearest ancestor of the button that has paint (opaque/semi fill, border or shadow) and also contains the name text node.',
        'Pass iff button.width ≥ 0.80 × card.width AND the left and right gaps between button and card edges differ by ≤ 2 px (full-width, symmetric).'],
      feedback: 'Not met: the Connect button must span the full width of the card.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const b = s.byRole('button', /connect/i)[0]; const name = first(s, S.name);
        if (!b) return cannot('no button named Connect');
        if (!name) return cannot('name text not found');
        const card = s.closest(s.parent(b) ?? b, n => hasPaint(n) && s.contains(n, name));
        if (!card) return bad('no painted card container around the button and name');
        const lg = b.rect[0] - card.rect[0], rg = (card.rect[0] + card.rect[2]) - (b.rect[0] + b.rect[2]);
        if (b.rect[2] < 0.8 * card.rect[2]) return bad(`button ${Math.round(b.rect[2])}px of card ${Math.round(card.rect[2])}px`);
        if (Math.abs(lg - rg) > 2) return bad(`asymmetric gaps ${Math.round(lg)}/${Math.round(rg)}`);
        return ok();
      },
    },
    {
      id: 'L01.4', kind: 'A', g2: 'Exact strings present',
      op: ['Snapshot at 1280 px. Each of the five strings "JD", "Jane Doe", "Product Designer", "Building the future of spatial computing via GPU pipelines.", "Connect" must be exactly the own text of at least one visible node (whitespace-collapsed, case-sensitive).'],
      feedback: 'Not met: the exact strings (initials, name, job title, bio, button label) must be present.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const miss = Object.values(S).filter(str => !first(s, str));
        return miss.length ? bad('missing: ' + miss.join(' | ')) : ok();
      },
    },
  ],
};
