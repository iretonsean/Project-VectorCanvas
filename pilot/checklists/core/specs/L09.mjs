import { R, ok, bad, cannot, norm, isRow, isColumn, iconsIn, iconOk } from './_common.mjs';
import { TEXT_INPUT_ROLES } from '../lib.mjs';

const textLink = (s, n) => n.role === 'link' && s.isVisible(n) && [n, ...s.descendants(n)].some(d => d.text && s.isVisible(d));
/** link columns: for each text link, the lowest ancestor containing >= 2 text links; distinct; outermost only; top 4 by link count */
export function linkGroups(s) {
  const links = s.nodes.filter(n => textLink(s, n));
  const groups = new Map();
  for (const l of links) {
    let c = s.parent(l);
    while (c && links.filter(x => s.contains(c, x)).length < 2) c = s.parent(c);
    if (c && c.kind !== 'body' && c.kind !== 'html') groups.set(c.id, c);
  }
  let arr = [...groups.values()];
  arr = arr.filter(a => !arr.some(b => b.id !== a.id && s.contains(a, b)));
  const cnt = g => links.filter(x => s.contains(g, x)).length;
  return arr.sort((a, b) => cnt(b) - cnt(a)).slice(0, 4).sort((a, b) => s.index.get(a.id) - s.index.get(b.id));
}
const nLinks = s => s.nodes.filter(n => textLink(s, n)).length;

export default {
  id: 'L09', category: '1 Layout-heavy', baseline: 'B1',
  intent: 'Footer with four link columns, newsletter signup, social icons, copyright line',
  fixture: ['The four link columns are titled Product, Company, Resources and Legal, each with three text links. The newsletter signup has an email field labelled "Email" and a "Subscribe" button. There are at least two social icons, each an icon-only link (no visible text; inline vector icons, no image files). The copyright line is plain text. On phones the four columns may be stacked or turned into an accordion.'],
  conventions: [
    'link column = lowest ancestor that contains ≥ 2 links with visible text; the 4 groups with the most links are the columns',
    'accordion = ≥ 4 visible buttons with an `expanded` state; activating one makes more text links visible',
    'icon-only link = visible link with no visible text in itself or its descendants',
  ],
  items: [
    {
      id: 'L09.1', kind: 'A', trace: true, g2: '4 columns collapse to an accordion or stack on phones',
      op: ['At 1280 and 1024 px: 4 link columns found and their rects form one row.',
        'At 480 and 320 px: either (stack) 4 link columns found and their rects form one column; or (accordion) fewer than 4 columns visible, ≥ 4 visible buttons with `expanded` = false, and activating the first increases the number of visible text links.'],
      feedback: 'Not met: the four link columns must sit in one row on wide screens and stack (or become an accordion) on phones.',
      async run(ctx) {
        for (const w of [1280, 1024]) {
          const s = await ctx.at(w); const g = linkGroups(s);
          if (g.length < 4) return cannot(`${w}px: found ${g.length} link columns (need 4)`);
          if (!isRow(g.map(x => x.rect))) return bad(`${w}px: the four columns are not in one row`);
        }
        for (const w of [480, 320]) {
          let s = await ctx.at(w); const g = linkGroups(s);
          if (g.length >= 4) { if (!isColumn(g.map(x => x.rect))) return bad(`${w}px: the four columns are neither stacked nor an accordion`); continue; }
          const tg = s.byRole('button').filter(b => b.states.expanded === false);
          if (tg.length < 4) return bad(`${w}px: ${g.length} columns visible and ${tg.length} accordion toggles`);
          const before = nLinks(s);
          await ctx.activate(tg[0]); s = await ctx.now();
          if (nLinks(s) <= before) return bad(`${w}px: activating the accordion toggle reveals no links`);
          await ctx.reload();
        }
        return ok();
      },
    },
    {
      id: 'L09.2', kind: 'A', g2: 'Icons have names',
      op: ['At 1280 px: ≥ 2 icon-only links (conventions); every one has a non-empty accessible name.',
        'Every icon in the page (visible svg/img, emoji-only text node) is decorative or named: hidden from the accessibility tree, or exposed as an `image` with a name, or inside a named link/button.'],
      feedback: 'Not met: every icon (including the social icons) must have an accessible name or be hidden from assistive technology.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const iconLinks = s.nodes.filter(n => n.role === 'link' && s.isVisible(n) && ![n, ...s.descendants(n)].some(d => d.text && s.isVisible(d)));
        if (iconLinks.length < 2) return cannot(`${iconLinks.length} icon-only links (need ≥ 2)`);
        const un = iconLinks.filter(l => !norm(l.name)); if (un.length) return bad(`${un.length} icon-only link(s) without a name`);
        const body = s.nodes.find(n => n.kind === 'body');
        const bad2 = iconsIn(s, body).filter(i => !iconOk(s, i));
        return bad2.length ? bad(`${bad2.length} icon(s) exposed without a name`) : ok();
      },
    },
    {
      id: 'L09.3', kind: 'A', g2: 'Email field labelled',
      op: ['At 1280 px: ≥ 1 visible text-input node (any text input role) exists, one of which has role `email-field` or a name matching /e-?mail/i. Every visible text input has a non-empty accessible name whose source is a label or an explicit name (`name_source` ∈ {label, aria}); a placeholder-only name does not count as a label.'],
      feedback: 'Not met: the email field must have a label.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const ins = s.byRole([...TEXT_INPUT_ROLES]);
        if (!ins.length) return bad('no text input');
        if (!ins.some(n => n.role === 'email-field' || /e-?mail/i.test(n.name ?? ''))) return bad('no email field');
        const un = ins.filter(n => !norm(n.name) || !['label', 'aria'].includes(n.name_source));
        return un.length ? bad(`${un.length} input(s) not labelled (source ${un[0].name_source})`) : ok();
      },
    },
  ],
};
