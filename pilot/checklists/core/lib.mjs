// core/lib.mjs — stack-neutral helpers over a Snapshot (see PVC_CHECKER_NEEDS.md §1).
// Nothing in here knows about the DOM or about PVC. Both stacks produce the same Snapshot JSON.

export const INTERACTIVE_ROLES = new Set([
  'button', 'link', 'checkbox', 'switch', 'radio', 'slider', 'stepper', 'text-field', 'text-area',
  'password-field', 'search-field', 'email-field', 'number-field', 'combo-box', 'list-box', 'option',
  'tab', 'menu-item', 'tree-item', 'grid-cell-interactive',
]);
export const TEXT_INPUT_ROLES = new Set([
  'text-field', 'text-area', 'password-field', 'search-field', 'email-field', 'number-field', 'stepper',
]);
// roles that count as "form controls" for label/control rows (L06)
export const CONTROL_ROLES = new Set([
  'checkbox', 'switch', 'radio', 'slider', 'stepper', 'text-field', 'text-area', 'password-field',
  'search-field', 'email-field', 'number-field', 'combo-box', 'list-box',
]);

export class Snap {
  constructor(raw) {
    this.raw = raw;
    this.meta = raw.meta;
    this.nodes = raw.nodes;
    this.byId = new Map();
    this.kidsOf = new Map();
    this.index = new Map();
    raw.nodes.forEach((n, i) => {
      this.byId.set(n.id, n);
      this.index.set(n.id, i);
      this.kidsOf.set(n.id, []);
    });
    for (const n of raw.nodes) {
      if (n.parent != null && this.kidsOf.has(n.parent)) this.kidsOf.get(n.parent).push(n);
    }
  }
  get vw() { return this.meta.viewport[0]; }
  get vh() { return this.meta.viewport[1]; }
  get(id) { return this.byId.get(id) ?? null; }
  parent(n) { return n.parent == null ? null : this.byId.get(n.parent) ?? null; }
  kids(n) { return this.kidsOf.get(n.id) ?? []; }
  ancestors(n) { const out = []; let p = this.parent(n); while (p) { out.push(p); p = this.parent(p); } return out; }
  descendants(n) {
    const out = []; const st = [...this.kids(n)].reverse();
    while (st.length) { const c = st.pop(); out.push(c); const k = this.kids(c); for (let i = k.length - 1; i >= 0; i--) st.push(k[i]); }
    return out;
  }
  contains(a, b) { // a is ancestor-or-self of b
    if (a.id === b.id) return true;
    let p = this.parent(b); while (p) { if (p.id === a.id) return true; p = this.parent(p); }
    return false;
  }
  before(a, b) { return this.index.get(a.id) < this.index.get(b.id); }
  all(pred) { return this.nodes.filter(pred); }
  isVisible(n) { return n.visible && n.rect[2] > 0 && n.rect[3] > 0 && n.visible_frac > 0; }
  /** nodes with the given role(s); optional name regexp (tested on accessible name) */
  byRole(roles, nameRe = null, { visibleOnly = true } = {}) {
    const rs = Array.isArray(roles) ? new Set(roles) : new Set([roles]);
    return this.nodes.filter(n => n.role && rs.has(n.role) && (!visibleOnly || this.isVisible(n)) && (!nameRe || (n.name != null && nameRe.test(n.name))));
  }
  /** nodes whose OWN text equals str (string) or matches re (RegExp) */
  byText(m, { visibleOnly = true } = {}) {
    return this.nodes.filter(n => n.text && (!visibleOnly || this.isVisible(n)) && (typeof m === 'string' ? n.text === m : m.test(n.text)));
  }
  textNodes({ visibleOnly = true } = {}) { return this.nodes.filter(n => n.text && (!visibleOnly || this.isVisible(n))); }
  visibleText() { return this.textNodes().map(n => n.text).join('\n'); }
  /** interactive nodes (role-based), visible, not disabled */
  interactive({ includeDisabled = false } = {}) {
    return this.nodes.filter(n => n.role && INTERACTIVE_ROLES.has(n.role) && this.isVisible(n) && !n.a11y_hidden && (includeDisabled || !n.states?.disabled));
  }
  /** lowest ancestor-or-self of n satisfying pred */
  closest(n, pred) { let c = n; while (c) { if (pred(c)) return c; c = this.parent(c); } return null; }
  /** the text blocks (nodes with own text) among descendants-or-self */
  textsWithin(n) { return [n, ...this.descendants(n)].filter(d => d.text && this.isVisible(d)); }
}

// ---------- geometry ----------
export const R = {
  x: r => r[0], y: r => r[1], w: r => r[2], h: r => r[3],
  r: r => r[0] + r[2], b: r => r[1] + r[3], cx: r => r[0] + r[2] / 2, cy: r => r[1] + r[3] / 2,
  area: r => Math.max(0, r[2]) * Math.max(0, r[3]),
  union(rs) {
    const rr = rs.filter(Boolean); if (!rr.length) return null;
    const x0 = Math.min(...rr.map(r => r[0])), y0 = Math.min(...rr.map(r => r[1]));
    const x1 = Math.max(...rr.map(r => r[0] + r[2])), y1 = Math.max(...rr.map(r => r[1] + r[3]));
    return [x0, y0, x1 - x0, y1 - y0];
  },
  inter(a, b) {
    const x0 = Math.max(a[0], b[0]), y0 = Math.max(a[1], b[1]);
    const x1 = Math.min(a[0] + a[2], b[0] + b[2]), y1 = Math.min(a[1] + a[3], b[1] + b[3]);
    return (x1 > x0 && y1 > y0) ? [x0, y0, x1 - x0, y1 - y0] : null;
  },
  interArea(a, b) { const i = R.inter(a, b); return i ? i[2] * i[3] : 0; },
  /** overlap length of [a0,a1] and [b0,b1] */
  ov1(a0, a1, b0, b1) { return Math.max(0, Math.min(a1, b1) - Math.max(a0, b0)); },
  hOverlap(a, b) { return R.ov1(a[0], a[0] + a[2], b[0], b[0] + b[2]); },
  vOverlap(a, b) { return R.ov1(a[1], a[1] + a[3], b[1], b[1] + b[3]); },
  /** a is entirely left of b (with tol slack) */
  leftOf(a, b, tol = 2) { return a[0] + a[2] <= b[0] + tol; },
  above(a, b, tol = 2) { return a[1] + a[3] <= b[1] + tol; },
  near(a, b, tol = 2) { return Math.abs(a - b) <= tol; },
};

// ---------- colour ----------
export function composite(fg, bg) { // fg=[r,g,b,a], bg=[r,g,b] opaque
  const a = fg[3] ?? 1;
  return [fg[0] * a + bg[0] * (1 - a), fg[1] * a + bg[1] * (1 - a), fg[2] * a + bg[2] * (1 - a)];
}
function lin(c) { c /= 255; return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; }
export function luminance([r, g, b]) { return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b); }
export function contrastRatio(c1, c2) {
  const l1 = luminance(c1), l2 = luminance(c2);
  const hi = Math.max(l1, l2), lo = Math.min(l1, l2);
  return (hi + 0.05) / (lo + 0.05);
}
/** effective opaque background behind node n: composite own fills walking up; page default white.
 *  returns {color:[r,g,b]} or {unknown:true} if a gradient/image sits behind. */
export function backdrop(snap, n) {
  const chain = [];
  let c = n;
  while (c) { chain.push(c); c = snap.parent(c); }
  let color = [255, 255, 255];
  for (let i = chain.length - 1; i >= 0; i--) {
    const s = chain[i].style;
    if (!s) continue;
    if (s.bg_unknown) return { unknown: true };
    if (s.bg && s.bg[3] > 0) color = composite(s.bg, color);
  }
  return { color };
}

// ---------- misc ----------
export const norm = s => (s ?? '').replace(/\s+/g, ' ').trim();
export const median = a => { if (!a.length) return NaN; const s = [...a].sort((x, y) => x - y); const m = s.length >> 1; return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2; };
export const pct = (a, p) => { const s = [...a].sort((x, y) => x - y); return s[Math.min(s.length - 1, Math.floor(p * s.length))]; };
export const hasPaint = n => !!(n.style && ((n.style.bg && n.style.bg[3] > 0.05) || n.style.bg_unknown || (n.style.border && n.style.border.w > 0) || n.style.shadow));
export const ok = (detail = '') => ({ pass: true, detail });
export const bad = detail => ({ pass: false, detail });
/** precondition not met: counted as FAIL, with a reason that the verbose log shows */
export const cannot = detail => ({ pass: false, unevaluable: true, detail });
export const EMOJI_RE = /\p{Extended_Pictographic}/u;
export function fmtRect(r) { return r ? `[${r.map(v => Math.round(v * 10) / 10).join(',')}]` : 'null'; }

/** isolate a container ("row"/"card") for a node: the largest ancestor-or-self of `n` that contains none of `others` */
export function largestExclusive(snap, n, others, stopAt = null) {
  let best = n; let c = snap.parent(n);
  while (c && (!stopAt || c.id !== stopAt.id)) {
    if (others.some(o => snap.contains(c, o))) break;
    best = c; c = snap.parent(c);
  }
  return best;
}
/** the lowest ancestor-or-self of n that contains n and at least one node of `want` and none of `avoid` */
export function lowestContaining(snap, n, want, avoid = []) {
  let c = n;
  while (c) {
    if (avoid.some(o => o.id !== n.id && snap.contains(c, o))) return null;
    if (want.some(w => snap.contains(c, w))) return c;
    c = snap.parent(c);
  }
  return null;
}
