// core/universal.mjs — universal checklist U1–U5 (G2 §3.1), operational definitions over the Snapshot.
import { INTERACTIVE_ROLES, R, backdrop, composite, contrastRatio, ok, bad, norm } from './lib.mjs';
import { Ctx } from './ctx.mjs';

export const WIDTHS = [320, 480, 768, 1024, 1280];

const ROVING = new Set(['tab', 'radio']);

export const U = [
  {
    id: 'U1', kind: 'A',
    g2: 'Every interactive element has an accessible name exposed to the platform accessibility API.',
    op: [
      'Snapshots at widths 320, 480, 768, 1024, 1280 (height 800, text scale 1).',
      'For every visible, enabled node whose `role` is interactive (button, link, checkbox, switch, radio, slider, stepper, any text input role, combo-box, list-box, option, tab, menu-item, tree-item): `name` is non-empty after trimming.',
      'Additionally, every visible node with `focusable` = true and `role` = null (a focusable element that exposes no role) fails, except the document root.',
      'Accessibility tree source: baseline = Chromium `Accessibility.getFullAXTree` (computed role/name); PVC = AccessKit-bound `role`/`name` in the Snapshot.',
    ],
    feedback: 'Not met: every interactive element must have an accessible name exposed to the accessibility API.',
    async run(ctx) {
      const probs = [];
      for (const w of WIDTHS) {
        const s = await ctx.at(w);
        for (const n of s.interactive()) if (!norm(n.name)) probs.push(`${w}px ${n.role} ${n.kind}#${n.id} has no name`);
        for (const n of s.nodes) if (n.focusable && !n.role && s.isVisible(n) && n.kind !== 'body' && n.kind !== 'html') probs.push(`${w}px focusable ${n.kind}#${n.id} exposes no role`);
      }
      return probs.length ? bad([...new Set(probs)].slice(0, 6).join('; ')) : ok();
    },
  },
  {
    id: 'U2', kind: 'A',
    g2: 'Every action is operable by keyboard alone.',
    op: [
      '(a) Reachability: from document start, press Tab repeatedly (≤200) and record the focused node each time. Every visible, enabled interactive node (same set as U1) must appear in the sequence at 1280 px and at 320 px. For roving groups (`tab`, `radio`) at least one member per group must be reached, the rest by arrow keys (checked in (b)). No keyboard trap: the Tab sequence must end (focus leaves or cycles) before 200 presses.',
      '(b) Trace replay: every scripted-interaction A item of the specimen that PASSED in pointer mode is re-run with `mode=keyboard`: `activate(node)` = reach the node with Tab/arrow keys, then Enter (button, link, tab) or Space (checkbox, switch, radio); `type(node, text)` = reach it with Tab, then type. The same expectations must hold. Items that failed in pointer mode are not double-counted here.',
    ],
    feedback: 'Not met: every action must be operable by keyboard alone.',
    async run(ctx) {
      const probs = [];
      for (const w of [1280, 320]) {
        const s = await ctx.at(w);
        const { order, trapped } = await ctx.focusOrder();
        if (trapped) probs.push(`${w}px keyboard trap (Tab sequence did not end)`);
        const reached = new Set(order);
        const seenGroups = new Set();
        for (const n of s.interactive()) {
          if (ROVING.has(n.role)) { // group-level
            const g = n.role + ':' + n.parent;
            if (reached.has(n.id)) seenGroups.add(g);
            continue;
          }
          if (['option', 'menu-item', 'tree-item'].includes(n.role)) continue;
          if (!reached.has(n.id)) probs.push(`${w}px ${n.role} "${norm(n.name)}" not reachable by Tab`);
        }
        for (const n of s.interactive()) if (ROVING.has(n.role)) {
          const g = n.role + ':' + n.parent;
          if (!seenGroups.has(g)) { probs.push(`${w}px ${n.role} group ${n.parent} has no Tab stop`); seenGroups.add(g); }
        }
      }
      // (b) keyboard replay of trace items that passed with the pointer
      const spec = ctx.spec;
      if (spec && ctx.driver) {
        for (const it of spec.items) {
          if (it.kind !== 'A' || !it.trace) continue;
          if (ctx.pointerResults[it.id] !== true) continue;
          const kc = new Ctx(ctx.driver, 'keyboard', ctx.pointerResults); kc.spec = spec;
          try {
            await kc.init();
            const r = await it.run(kc);
            if (!r.pass) probs.push(`keyboard replay of ${it.id} failed: ${r.detail}`);
          } catch (e) { probs.push(`keyboard replay of ${it.id} error: ${e.message.split('\n')[0]}`); }
          finally { await kc.close(); }
        }
      }
      return probs.length ? bad(probs.slice(0, 6).join('; ')) : ok();
    },
  },
  {
    id: 'U3', kind: 'A',
    g2: 'Text contrast meets WCAG 2.2 AA.',
    op: [
      'Snapshots at 1280 px and 320 px. Colours are the resolved `style.fg` and the backdrop composited from the nearest chain of `style.bg` fills up to white page background (alpha composited; ancestor `style.opacity` multiplies fg alpha).',
      'Every visible text node (own `text` non-empty, `visible_frac`>0, not disabled): contrast ratio ≥ 4.5:1, or ≥ 3:1 for large text (font size ≥ 24 px, or ≥ 18.66 px with weight ≥ 700). Ratio = (L1+0.05)/(L2+0.05), WCAG relative luminance.',
      'UI components (non-text, WCAG 1.4.11): for text inputs, combo-box and custom (non-`native`) checkbox/radio/switch/slider: the control boundary (`style.border` colour if width ≥ 1, else the node\'s own fill) against its parent backdrop ≥ 3:1. Buttons that have a text label and natively drawn controls are exempt.',
      'Text over a gradient/image (`bg_unknown`) cannot be resolved: it is skipped and counted in the verbose log (not a failure).',
      'Disabled controls and their text are exempt (WCAG inactive component). Placeholder text is not examined (not in the Snapshot).',
    ],
    feedback: 'Not met: text contrast must meet WCAG 2.2 AA (4.5:1 text, 3:1 large text and UI components).',
    async run(ctx) {
      const probs = []; let skipped = 0;
      for (const w of [1280, 320]) {
        const s = await ctx.at(w);
        for (const n of s.textNodes()) {
          if (n.states?.disabled || s.ancestors(n).some(a => a.states?.disabled)) continue;
          const bd = backdrop(s, n);
          if (bd.unknown) { skipped++; continue; }
          const op = n.style.opacity ?? 1;
          const fg = composite([n.style.fg[0], n.style.fg[1], n.style.fg[2], (n.style.fg[3] ?? 1) * op], bd.color);
          // a node's own fill sits behind its own text: backdrop() already includes the node itself
          const ratio = contrastRatio(fg, bd.color);
          const large = n.style.font_size >= 24 || (n.style.font_size >= 18.66 && n.style.font_weight >= 700);
          const need = large ? 3 : 4.5;
          if (ratio < need - 1e-9) probs.push(`${w}px text "${n.text.slice(0, 24)}" ${ratio.toFixed(2)}:1 < ${need}`);
        }
        for (const n of s.interactive()) {
          if (n.native) continue;
          const isInput = ['text-field', 'text-area', 'password-field', 'search-field', 'email-field', 'number-field', 'combo-box', 'stepper'].includes(n.role);
          const isCustomToggle = ['checkbox', 'radio', 'switch', 'slider'].includes(n.role);
          if (!isInput && !isCustomToggle) continue;
          const par = s.parent(n); const pb = par ? backdrop(s, par) : { color: [255, 255, 255] };
          if (pb.unknown) { skipped++; continue; }
          let edge = null;
          if (n.style.border && n.style.border.w >= 1 && n.style.border.color[3] > 0) edge = composite(n.style.border.color, pb.color);
          else if (n.style.bg && n.style.bg[3] > 0) edge = composite(n.style.bg, pb.color);
          if (!edge) { probs.push(`${w}px ${n.role} "${norm(n.name)}" has no visible boundary`); continue; }
          const ratio = contrastRatio(edge, pb.color);
          if (ratio < 3 - 1e-9) probs.push(`${w}px ${n.role} "${norm(n.name)}" boundary ${ratio.toFixed(2)}:1 < 3`);
        }
      }
      ctx.log.push(`U3 skipped (gradient/image backdrop): ${skipped}`);
      return probs.length ? bad([...new Set(probs)].slice(0, 6).join('; ')) : ok(skipped ? `skipped ${skipped} unresolved` : '');
    },
  },
  {
    id: 'U4', kind: 'A',
    g2: 'At a 320-px-wide viewport there is no overlap, clipping of meaningful content, or horizontal scroll (unless the specimen asks for it).',
    op: [
      'Snapshot at viewport 320×800, text scale 1 (see `u4Check`).',
      '(i) Horizontal scroll: `meta.scroll_overflow_x` ≤ 1 px.',
      '(ii) Clipping: every visible node with own text, every visible interactive node and every visible non-decorative `image` has `visible_frac` ≥ 0.95 (i.e. not cut off by a clip container, by an ellipsis/line-clamp truncation, or by the viewport\'s left/right edges).',
      '(iii) Overlap: content boxes = per-line text boxes, interactive-node rects, non-decorative image rects. Two boxes from different nodes where neither node is an ancestor of the other, and whose intersection area ≥ 25% of the smaller box (and ≥ 3 px in both dimensions), is an overlap. Purely decorative nodes (`a11y_hidden` with no text) are ignored.',
      'No specimen in the pilot set asks for horizontal scroll.',
    ],
    feedback: 'Not met: at a 320-px-wide viewport there must be no overlap, clipping of meaningful content, or horizontal scroll.',
    async run(ctx) { return u4Check(ctx, 1, 320); },
  },
  {
    id: 'U5', kind: 'A',
    g2: 'At 200% text scale, U4 still holds.',
    op: [
      'Same as U4 but with text scale 2.0 at 320 px width. Baseline: the checker sets `html{font-size:200% !important}` (every rem/em/% sized length scales; px-sized type does not — nearest a web author can be tested against; stated deviation). PVC: `text_scale: 2.0` scales all text.',
    ],
    feedback: 'Not met: at 200% text scale, there must still be no overlap, clipping of meaningful content, or horizontal scroll at a 320-px-wide viewport.',
    async run(ctx) { return u4Check(ctx, 2, 320); },
  },
];

export async function u4Check(ctx, scale, w) {
  const s = await ctx.at(w, { scale });
  const probs = [];
  if (s.meta.scroll_overflow_x > 1) probs.push(`horizontal scroll ${Math.round(s.meta.scroll_overflow_x)}px`);
  for (const n of s.nodes) {
    const isImg = n.visual && !n.a11y_hidden && n.role === 'image';
    if (!(n.text || (n.role && INTERACTIVE_ROLES.has(n.role)) || isImg)) continue;
    if (n.a11y_hidden && !n.text) continue;
    if (!n.visible) { if (n.text && n.rect[2] > 1 && n.rect[3] > 1 && n.style.opacity > 0.01) probs.push(`clipped text "${n.text.slice(0, 20)}"`); continue; }
    if (n.visible_frac < 0.95) probs.push(`clipped ${n.role ?? n.kind} "${(n.text || n.name || '').slice(0, 20)}" (${Math.round(n.visible_frac * 100)}% visible)`);
  }
  // overlaps
  const boxes = [];
  for (const n of s.nodes) {
    if (!s.isVisible(n)) continue;
    if (n.a11y_hidden && !n.text) continue;
    for (const l of n.lines) boxes.push({ n, r: l });
    if (n.role && INTERACTIVE_ROLES.has(n.role)) boxes.push({ n, r: n.rect });
    if (n.visual && !n.a11y_hidden && n.role === 'image') boxes.push({ n, r: n.rect });
  }
  let found = 0;
  for (let i = 0; i < boxes.length && found < 4; i++) for (let j = i + 1; j < boxes.length && found < 4; j++) {
    const a = boxes[i], b = boxes[j];
    if (a.n.id === b.n.id) continue;
    if (s.contains(a.n, b.n) || s.contains(b.n, a.n)) continue;
    const it = R.inter(a.r, b.r); if (!it) continue;
    if (it[2] < 3 || it[3] < 3) continue;
    const small = Math.min(R.area(a.r), R.area(b.r));
    if (it[2] * it[3] >= 0.25 * small) { found++; probs.push(`overlap "${(a.n.text || a.n.name || a.n.kind).slice(0, 16)}" × "${(b.n.text || b.n.name || b.n.kind).slice(0, 16)}"`); }
  }
  return probs.length ? bad([...new Set(probs)].slice(0, 6).join('; ')) : ok();
}
