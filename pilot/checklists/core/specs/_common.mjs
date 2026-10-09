// core/specs/_common.mjs — predicates shared by several specimens.
import { R, ok, bad, cannot, norm, hasPaint, median, EMOJI_RE } from '../lib.mjs';

export const W = { phone: 320, phone2: 480, tablet: 768, laptop: 1024, desktop: 1280 };

/** rects sorted left→right form ONE ROW: pairwise x-disjoint (tol) and each adjacent pair vertically overlaps ≥50% of the shorter */
export function isRow(rects, tol = 2) {
  const rs = [...rects].sort((a, b) => a[0] - b[0]);
  for (let i = 0; i + 1 < rs.length; i++) {
    if (!R.leftOf(rs[i], rs[i + 1], tol)) return false;
    if (R.vOverlap(rs[i], rs[i + 1]) < 0.5 * Math.min(rs[i][3], rs[i + 1][3])) return false;
  }
  return true;
}
/** rects sorted top→bottom form ONE COLUMN: y-disjoint (tol) and adjacent pairs overlap ≥ 60% horizontally of the narrower */
export function isColumn(rects, tol = 2) {
  const rs = [...rects].sort((a, b) => a[1] - b[1]);
  for (let i = 0; i + 1 < rs.length; i++) {
    if (!R.above(rs[i], rs[i + 1], tol)) return false;
    if (R.hOverlap(rs[i], rs[i + 1]) < 0.6 * Math.min(rs[i][2], rs[i + 1][2])) return false;
  }
  return true;
}
/** group rects into visual rows (by vertical overlap), return rows sorted top→bottom, each sorted left→right */
export function rowsOf(rects) {
  const rs = [...rects].sort((a, b) => a[1] - b[1] || a[0] - b[0]);
  const rows = [];
  for (const r of rs) {
    const row = rows.find(q => R.vOverlap(q.bbox, r) >= 0.5 * Math.min(q.bbox[3], r[3]));
    if (row) { row.items.push(r); row.bbox = R.union(row.items); } else rows.push({ items: [r], bbox: r });
  }
  rows.forEach(r => r.items.sort((a, b) => a[0] - b[0]));
  return rows.map(r => r.items);
}
export const shapeOf = rects => rowsOf(rects).map(r => r.length); // e.g. [3,3] or [1,1,1,1,1,1]

export const CTA_ROLES = ['button', 'link'];
export const PRICE_RE = /^\s*(?:[$€£¥]\s?\d|\d[\d.,]*\s?(?:[$€£¥]|USD|EUR|GBP)|free\s*$)/i;
export const isPriceNode = n => n.text && n.text.length <= 30 && PRICE_RE.test(n.text);

/** icons inside container c: svg/img nodes plus emoji-only text nodes. */
export function iconsIn(s, c) {
  const out = [];
  for (const d of [c, ...s.descendants(c)]) {
    if (d.visual === 'graphic' || d.visual === 'image') out.push({ n: d, kind: d.visual });
    else if (d.text && EMOJI_RE.test(d.text) && d.text.replace(/\p{Extended_Pictographic}|\uFE0F|\u200D|\s/gu, '') === '') out.push({ n: d, kind: 'emoji' });
  }
  return out;
}
/** decorative-or-named: hidden from the accessibility tree, or exposed as an image/with a non-empty name */
export function iconOk(s, ic) {
  const n = ic.n;
  if (n.a11y_hidden) return true;
  if (n.role === 'image' && norm(n.name)) return true;
  // icon inside a named interactive element where the name carries the meaning
  const host = s.closest(n, a => a.role && ['link', 'button'].includes(a.role));
  if (host && host.id !== n.id && norm(host.name) && n.role !== 'image') return true;
  return false;
}

export function visibleControls(s, roles) { return s.byRole(roles).filter(n => !n.a11y_hidden); }

/** a modal/prompt exposed in-page */
export function inPageDialog(s) {
  return s.nodes.filter(n => ['dialog', 'alert-dialog'].includes(n.role) && s.isVisible(n));
}
export { ok, bad, cannot, norm, hasPaint, median, R, EMOJI_RE };

// ---------- state-trace helpers (G specimens) ----------
export const btn = (s, re) => s.byRole('button', re)[0] ?? null;
export const anyCtl = (s, re, roles = ['button', 'link', 'tab', 'radio']) => s.byRole(roles, re)[0] ?? null;
/** row of a task/line identified by its visible text: the largest ancestor containing that text and >= 1 control but none of the other texts */
export function rowOfText(s, str, others = [], controlRoles = ['button', 'checkbox', 'link', 'switch', 'radio']) {
  const t = s.byText(str)[0]; if (!t) return null;
  const oth = others.flatMap(o => s.byText(o));
  let c = t; let best = null;
  while (c && c.kind !== 'body' && c.kind !== 'html') {
    if (oth.some(o => s.contains(c, o))) break;
    if (s.descendants(c).some(d => controlRoles.includes(d.role) && s.isVisible(d))) best = c;
    c = s.parent(c);
  }
  if (!best) return null;
  return { text: t, row: best, within: role => s.descendants(best).filter(d => (Array.isArray(role) ? role.includes(d.role) : d.role === role) && s.isVisible(d)) };
}
export const pageText = s => s.textNodes().map(n => n.text);
export function parseMoney(str) { const m = /(-?)\$?\s*(-?)([\d,]+)\.(\d{2})\b/.exec(str ?? ''); if (!m) return null; const v = parseInt(m[3].replace(/,/g, ''), 10) * 100 + parseInt(m[4], 10); return (m[1] || m[2]) ? -v : v; }
/** amount that goes with a label: in the label's own text, else the next money text in document order within two ancestor levels */
export function labelAmount(s, labelRe) {
  const l = s.nodes.find(n => s.isVisible(n) && n.text && labelRe.test(n.text));
  if (!l) return undefined;
  const own = parseMoney(l.text); if (own != null) return own;
  let box = s.parent(l);
  for (let lvl = 0; lvl < 3 && box; lvl++, box = s.parent(box)) {
    const cand = [box, ...s.descendants(box)].filter(n => s.isVisible(n) && n.text && n.id !== l.id && s.index.get(n.id) > s.index.get(l.id) && parseMoney(n.text) != null);
    if (cand.length) return parseMoney(cand[0].text);
  }
  return null;
}
