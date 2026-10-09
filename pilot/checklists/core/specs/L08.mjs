import { R, ok, bad, cannot, norm, iconsIn, iconOk, EMOJI_RE } from './_common.mjs';

const MON = '(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\\.?';
const DAY = '(?:mon|tue|wed|thu|fri|sat|sun)[a-z]*\\.?';
const DATE_TOKEN = new RegExp(`^(?:${MON}|${DAY}|\\d{1,2})$`, 'i');
const DATE_FULL = new RegExp(`${MON}\\s+\\d{1,2}|\\d{1,2}\\s+${MON}`, 'i');
const isDateNode = n => n.text && (DATE_TOKEN.test(n.text) || DATE_FULL.test(n.text)) && n.text.length <= 14;

export function eventParts(s) {
  const dates = s.nodes.filter(n => s.isVisible(n) && isDateNode(n));
  const tc = s.nodes.filter(n => s.isVisible(n) && n.text && !isDateNode(n) && !(n.role && ['link', 'button'].includes(n.role)));
  const heading = s.byRole('heading')[0] ?? tc.sort((a, b) => b.style.font_size - a.style.font_size)[0];
  return { dates, heading, dateRect: dates.length ? R.union(dates.map(d => d.rect)) : null };
}

export default {
  id: 'L08', category: '1 Layout-heavy', baseline: 'B1',
  intent: 'Event poster-style card: date block on the left, title, location with icon, "Add to calendar" link',
  fixture: ['The date block shows a short month and a day number as separate lines (for example "OCT" and "12"). The card also has an event title (a heading), a location line with a small icon (an inline vector icon, no image files), and a link labelled "Add to calendar".'],
  conventions: [
    'date tokens = visible text nodes that are a month abbreviation/name, a weekday abbreviation/name, or a 1–2 digit number (≤ 14 chars); the date block rect = union of those nodes',
    'title = the first heading, else the text node with the largest font size that is not a date token',
    'RTL is applied by the checker as direction:"rtl" (baseline: `dir="rtl"` on <html>; PVC: `direction:"rtl"` in the Snapshot request)',
  ],
  items: [
    {
      id: 'L08.1', kind: 'A', g2: 'Date block left on LTR and right on RTL locales',
      op: ['At 1280 px LTR: date block and title located; date block right edge ≤ title left edge + 2 px and they overlap vertically.',
        'At 1280 px with direction RTL: date block left edge ≥ title right edge − 2 px and they overlap vertically.'],
      feedback: 'Not met: the date block must be on the left in left-to-right languages and on the right in right-to-left languages.',
      async run(ctx) {
        for (const dir of ['ltr', 'rtl']) {
          const s = await ctx.at(1280, { dir });
          const { dateRect, heading } = eventParts(s);
          if (!dateRect || !heading) return cannot(`${dir}: date block or title not found`);
          const hr = heading.lines.length ? R.union(heading.lines) : heading.rect;
          if (R.vOverlap(dateRect, hr) <= 0) return bad(`${dir}: date block and title not on one row`);
          if (dir === 'ltr' && !(R.r(dateRect) <= hr[0] + 2)) return bad('ltr: date block not left of title');
          if (dir === 'rtl' && !(dateRect[0] >= R.r(hr) - 2)) return bad('rtl: date block not right of title');
        }
        return ok();
      },
    },
    {
      id: 'L08.2', kind: 'A', g2: 'Icon decorative',
      op: ['At 1280 px: the card (the nearest ancestor of the title that also contains the date block) contains ≥ 1 icon (visible svg/img node, or an emoji-only text node).',
        'Every icon is decorative or named: hidden from the accessibility tree (`a11y_hidden`), or an `image` with a non-empty name, or inside a named link/button. An emoji pictograph inside a text node that also contains other characters fails (it would be read out).',
        'Pass iff ≥ 1 icon exists and every icon is decorative-or-named.'],
      feedback: 'Not met: the location icon must be present and decorative (hidden from assistive technology).',
      async run(ctx) {
        const s = await ctx.at(1280);
        const { dates, heading } = eventParts(s);
        if (!heading || !dates.length) return cannot('date block or title not found');
        const card = s.closest(heading, n => dates.every(d => s.contains(n, d)));
        if (!card) return cannot('no common container');
        const icons = iconsIn(s, card);
        const mixedEmoji = [card, ...s.descendants(card)].filter(n => n.text && EMOJI_RE.test(n.text) && n.text.replace(/\p{Extended_Pictographic}|\uFE0F|\u200D|\s/gu, '') !== '');
        if (!icons.length && !mixedEmoji.length) return bad('no icon found');
        if (mixedEmoji.length) return bad('emoji inside readable text, not a separate decorative node');
        const notOk = icons.filter(i => !iconOk(s, i));
        return notOk.length ? bad(`${notOk.length} icon(s) exposed to assistive technology without being named`) : ok();
      },
    },
    {
      id: 'L08.3', kind: 'A', g2: 'Link role is link',
      op: ['At 1280 px: a visible node whose accessible name matches /add to calendar/i has role `link`.'],
      feedback: 'Not met: "Add to calendar" must be a link.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const c = s.nodes.filter(n => s.isVisible(n) && n.role && n.name && /add to calendar/i.test(n.name) && ['link', 'button'].includes(n.role));
        if (!c.length) return bad('no control named "Add to calendar"');
        return c.some(n => n.role === 'link') ? ok() : bad(`role is ${c[0].role}`);
      },
    },
  ],
};
