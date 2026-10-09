// baseline/probe-browser.mjs — produces the stack-neutral Snapshot (PVC_CHECKER_NEEDS.md §1) from Chromium.
// DOM walk in the page (geometry, text, colours) + CDP Accessibility.getFullAXTree (role, name, states).
import { chromium } from 'playwright';

const ROLE_MAP = {
  button: 'button', link: 'link', checkbox: 'checkbox', switch: 'switch', radio: 'radio', radiogroup: 'radio-group',
  slider: 'slider', spinbutton: 'stepper', textbox: 'text-field', searchbox: 'search-field', combobox: 'combo-box',
  listbox: 'list-box', option: 'option', list: 'list', listitem: 'list-item', table: 'table', row: 'row', cell: 'cell',
  columnheader: 'column-header', rowheader: 'column-header', grid: 'grid', gridcell: 'grid-cell', tablist: 'tab-list',
  tab: 'tab', tabpanel: 'tab-panel', dialog: 'dialog', alertdialog: 'alert-dialog', alert: 'alert', status: 'status',
  heading: 'heading', paragraph: 'paragraph', labeltext: 'label', legend: 'label', image: 'image', img: 'image',
  progressbar: 'progress', meter: 'meter', menu: 'menu', menuitem: 'menu-item', toolbar: 'toolbar', tooltip: 'tooltip',
  navigation: 'navigation', main: 'main', region: 'region', form: 'form', group: 'group', banner: 'banner',
  contentinfo: 'contentinfo', complementary: 'complementary', article: 'article', blockquote: 'blockquote',
  figure: 'figure', separator: 'separator', tree: 'tree', treeitem: 'tree-item', search: 'search', term: 'term',
  definition: 'definition', code: 'code', time: 'time', emphasis: 'emphasis', strong: 'strong', mark: 'mark',
  description: 'description', 'description list': 'description-list', 'list marker': 'list-marker',
  details: 'details', disclosuretriangle: 'button', menulistpopup: 'listbox', menulistoption: 'option',
  popupbutton: 'combo-box', // <select> in older Chromium builds
};
const NON_ROLES = new Set(['generic', 'none', 'presentation', 'statictext', 'inlinetextbox', 'rootwebarea', 'webarea',
  'linebreak', 'ignored', 'unknown', 'genericcontainer', 'section', 'div', 'text', 'document', 'iframe']);

/** in-page extractor (stringified into the page) */
function pageExtract() {
  const SKIP = new Set(['SCRIPT', 'STYLE', 'NOSCRIPT', 'TEMPLATE', 'LINK', 'META', 'TITLE', 'HEAD', 'BASE']);
  const sx = window.scrollX, sy = window.scrollY;
  const vw = window.innerWidth;
  window.__pvcN = window.__pvcN || 0;
  const cvs = document.createElement('canvas'); cvs.width = cvs.height = 1;
  const cctx = cvs.getContext('2d', { willReadFrequently: true });
  const colCache = new Map();
  function parseColor(str) {
    if (colCache.has(str)) return colCache.get(str);
    let out;
    const m = /^rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:\s*[,/]\s*([\d.]+%?))?\s*\)$/.exec(str);
    if (m) {
      let a = m[4] === undefined ? 1 : (m[4].endsWith('%') ? parseFloat(m[4]) / 100 : parseFloat(m[4]));
      out = [Math.round(+m[1]), Math.round(+m[2]), Math.round(+m[3]), a];
    } else if (str === 'transparent') out = [0, 0, 0, 0];
    else {
      cctx.clearRect(0, 0, 1, 1); cctx.fillStyle = '#000'; cctx.fillStyle = str; cctx.fillRect(0, 0, 1, 1);
      const d = cctx.getImageData(0, 0, 1, 1).data;
      out = [d[0], d[1], d[2], d[3] / 255];
    }
    colCache.set(str, out); return out;
  }
  const px = (v, base) => { if (v.endsWith('%')) return parseFloat(v) / 100 * base; const f = parseFloat(v); return isNaN(f) ? 0 : f; };
  const collapse = s => s.replace(/\s+/g, ' ').trim();
  const out = [];

  function textInfo(el) {
    let text = ''; const lines = []; const chars = [];
    for (const tn of el.childNodes) {
      if (tn.nodeType !== 3) continue;
      const raw = tn.nodeValue;
      if (!/\S/.test(raw)) continue;
      text += (text ? ' ' : '') + collapse(raw);
      const rg = document.createRange(); rg.selectNodeContents(tn);
      const rects = [...rg.getClientRects()].filter(r => r.width > 0 && r.height > 0);
      // merge fragments on the same visual line
      const ls = [];
      for (const r of rects) {
        const l = ls.find(q => Math.abs((q.y + q.h / 2) - (r.top + r.height / 2)) < Math.min(q.h, r.height) / 2);
        if (l) { const x0 = Math.min(l.x, r.left), x1 = Math.max(l.x + l.w, r.right); l.x = x0; l.w = x1 - x0; l.y = Math.min(l.y, r.top); l.h = Math.max(l.h, r.height); }
        else ls.push({ x: r.left, y: r.top, w: r.width, h: r.height });
      }
      ls.sort((a, b) => a.y - b.y || a.x - b.x);
      if (ls.length <= 1) { chars.push(collapse(raw).length); }
      else {
        // per-character line assignment
        const per = ls.map(() => []);
        for (let i = 0; i < raw.length; i++) {
          rg.setStart(tn, i); rg.setEnd(tn, i + 1);
          const rs = [...rg.getClientRects()].filter(r => r.width > 0 && r.height > 0);
          if (!rs.length) continue;
          const cy = rs[0].top + rs[0].height / 2;
          let k = ls.findIndex(q => cy >= q.y && cy <= q.y + q.h);
          if (k < 0) k = 0;
          per[k].push(i);
        }
        for (const idxs of per) {
          if (!idxs.length) { chars.push(0); continue; }
          chars.push(collapse(raw.slice(idxs[0], idxs[idxs.length - 1] + 1)).length);
        }
      }
      for (const l of ls) lines.push([l.x + sx, l.y + sy, l.w, l.h]);
    }
    return { text, lines, chars };
  }

  function visit(el, parentId, ancOpacity, clipRects, ariaHidden) {
    if (SKIP.has(el.tagName)) return;
    const cs = getComputedStyle(el);
    if (cs.display === 'none') return;
    const tag = el.tagName.toLowerCase();
    const isSvg = el instanceof SVGElement;
    if (isSvg && tag !== 'svg') return; // svg internals are not separate nodes
    let thisId = parentId;
    const opac = ancOpacity * parseFloat(cs.opacity || '1');
    const ah = ariaHidden || el.getAttribute('aria-hidden') === 'true';
    let nextClips = clipRects;
    const rectDom = el.getBoundingClientRect();
    if (cs.display !== 'contents' && cs.visibility !== 'hidden') {
      if (!el.hasAttribute('data-pvc-id')) el.setAttribute('data-pvc-id', 'n' + (++window.__pvcN));
      thisId = el.getAttribute('data-pvc-id');
      const rect = [rectDom.left + sx, rectDom.top + sy, rectDom.width, rectDom.height];
      const ti = textInfo(el);
      const minDim = Math.min(rect[2], rect[3]);
      let visual = null;
      if (tag === 'img' || tag === 'video' || tag === 'canvas' || tag === 'picture') visual = 'image';
      else if (isSvg) visual = 'graphic';
      // clip (own overflow clips own text lines; ancestors' clips apply to everything)
      const ownClip = (cs.overflowX !== 'visible' || cs.overflowY !== 'visible') && tag !== 'html' && tag !== 'body'
        ? (() => {
          const cx0 = cs.overflowX !== 'visible' ? rect[0] : -1e9, cx1 = cs.overflowX !== 'visible' ? rect[0] + rect[2] : 2e9;
          const cy0 = cs.overflowY !== 'visible' ? rect[1] : -1e9, cy1 = cs.overflowY !== 'visible' ? rect[1] + rect[3] : 2e9;
          return [cx0, cy0, cx1 - cx0, cy1 - cy0];
        })() : null;
      const useClips = ti.lines.length && ownClip ? [...clipRects, ownClip] : clipRects;
      const target = ti.lines.length ? ti.lines : [rect];
      let minFrac = 1;
      for (const t of target) {
        const a = t[2] * t[3]; if (a <= 0) continue;
        let x0 = Math.max(t[0], 0), x1 = Math.min(t[0] + t[2], vw), y0 = t[1], y1 = t[1] + t[3];
        for (const c of useClips) { x0 = Math.max(x0, c[0]); x1 = Math.min(x1, c[0] + c[2]); y0 = Math.max(y0, c[1]); y1 = Math.min(y1, c[1] + c[3]); }
        const vis = (x1 > x0 && y1 > y0) ? (x1 - x0) * (y1 - y0) / a : 0;
        minFrac = Math.min(minFrac, vis);
      }
      const bw = Math.max(parseFloat(cs.borderTopWidth) || 0, parseFloat(cs.borderRightWidth) || 0, parseFloat(cs.borderBottomWidth) || 0, parseFloat(cs.borderLeftWidth) || 0);
      const bstyle = [cs.borderTopStyle, cs.borderRightStyle, cs.borderBottomStyle, cs.borderLeftStyle];
      let bcol = null;
      if (bw > 0 && bstyle.some(s => s !== 'none' && s !== 'hidden')) {
        const sides = [['Top', cs.borderTopWidth], ['Right', cs.borderRightWidth], ['Bottom', cs.borderBottomWidth], ['Left', cs.borderLeftWidth]];
        const s = sides.find(s => parseFloat(s[1]) > 0) || sides[0];
        bcol = parseColor(cs['border' + s[0] + 'Color']);
      }
      const radii = [cs.borderTopLeftRadius, cs.borderTopRightRadius, cs.borderBottomLeftRadius, cs.borderBottomRightRadius].map(v => px(v.split(' ')[0], rect[2]));
      const bg = parseColor(cs.backgroundColor);
      const bgImg = cs.backgroundImage && cs.backgroundImage !== 'none';
      const isInput = tag === 'input' || tag === 'textarea' || tag === 'select';
      const itype = tag === 'input' ? (el.getAttribute('type') || 'text').toLowerCase() : '';
      const nat = tag === 'input' && ['checkbox', 'radio', 'range', 'color', 'file'].includes(itype) && cs.appearance !== 'none';
      const visible = cs.visibility !== 'hidden' && opac > 0.01 && rect[2] > 0 && rect[3] > 0 && !(ti.text && (minFrac < 0.05 || rect[2] <= 1 || rect[3] <= 1));
      let focusable = false;
      try { focusable = el.tabIndex >= 0 && !el.disabled && !el.closest('[inert]') && visible; } catch (e) { /* */ }
      if (tag === 'body' || tag === 'html') focusable = false;
      const node = {
        id: thisId, parent: parentId, kind: tag, role: null, name: null, name_source: null, states: {},
        a11y_hidden: ah, visible, text: ti.text, lines: ti.lines, line_chars: ti.chars,
        rect, visible_frac: Math.round(minFrac * 1000) / 1000, focusable,
        style: {
          fg: parseColor(cs.color), bg: bg[3] > 0 ? bg : null, bg_unknown: !!bgImg,
          border: bcol ? { w: bw, color: bcol } : null, radius: Math.min(Math.max(...radii), minDim / 2),
          font_size: parseFloat(cs.fontSize), font_weight: parseInt(cs.fontWeight, 10) || 400,
          shadow: cs.boxShadow !== 'none', opacity: opac, direction: cs.direction,
        },
        visual, native: nat,
        attrs: { ariaLabel: el.getAttribute('aria-label'), role: el.getAttribute('role'), type: itype || null, alt: el.getAttribute('alt'), href: el.getAttribute('href') },
      };
      if (isInput) node.value = el.value;
      out.push(node);
      if (ownClip) nextClips = [...clipRects, ownClip];
    }
    if (isSvg) return;
    for (const ch of el.children) visit(ch, thisId, opac, nextClips, ah);
  }
  visit(document.body, null, 1, [], false);
  const de = document.documentElement;
  const sw = Math.max(de.scrollWidth, document.body.scrollWidth);
  const sh = Math.max(de.scrollHeight, document.body.scrollHeight);
  const ae = document.activeElement; const act = ae && ae !== document.body && ae !== document.documentElement && ae.getAttribute ? ae.getAttribute('data-pvc-id') : null;
  return { nodes: out, doc: [sw, sh], overflowX: Math.max(0, sw - vw), focused: act, lang: de.lang || '' };
}

function axProps(n) {
  const p = {};
  for (const q of n.properties || []) p[q.name] = q.value ? q.value.value : undefined;
  return p;
}
function nameSource(n) {
  const srcs = n.name?.sources || [];
  const used = srcs.find(s => s.value && s.value.value !== undefined && s.value.value !== '' && !s.superseded);
  if (!used) return 'none';
  if (used.type === 'relatedElement') return used.attribute === 'aria-labelledby' ? 'aria' : 'label';
  if (used.type === 'attribute') {
    if (used.attribute === 'aria-label') return 'aria';
    if (used.attribute === 'placeholder') return 'placeholder';
    if (used.attribute === 'title') return 'title';
    if (used.attribute === 'alt') return 'aria';
    return 'attr:' + used.attribute;
  }
  if (used.type === 'placeholder') return 'placeholder';
  if (used.type === 'contents') return 'content';
  if (used.type === 'relatedElement' || used.nativeSource === 'label' || used.nativeSource === 'labelfor' || used.nativeSource === 'labelwrapped') return 'label';
  return String(used.type);
}

export class BrowserProbe {
  /** page: playwright Page; cdp: CDPSession */
  constructor(page, cdp, ctxInfo) {
    this.page = page; this.cdp = cdp; this.dialogs = []; this.dialogPolicy = 'dismiss';
    this.viewport = [1280, 800]; this.scale = 1; this.dir = 'ltr';
    page.on('dialog', async d => {
      this.dialogs.push({ type: d.type(), message: d.message() });
      if (this.dialogPolicy === 'accept') await d.accept(); else await d.dismiss();
    });
  }
  async setView({ w = 1280, h = 800, scale = 1, dir = 'ltr' } = {}) {
    const [cw, ch] = this.viewport;
    if (cw !== w || ch !== h) await this.page.setViewportSize({ width: w, height: h });
    this.viewport = [w, h];
    if (scale !== this.scale || dir !== this.dir || true) {
      await this.page.evaluate(({ scale, dir }) => {
        let st = document.getElementById('__pvc_scale');
        if (scale !== 1) {
          if (!st) { st = document.createElement('style'); st.id = '__pvc_scale'; document.head.appendChild(st); }
          st.textContent = `html{font-size:${scale * 100}% !important}`;
        } else if (st) st.remove();
        if (document.documentElement.getAttribute('dir') !== (dir === 'rtl' ? 'rtl' : null) && !(dir === 'ltr' && !document.documentElement.getAttribute('dir'))) {
          if (dir === 'rtl') document.documentElement.setAttribute('dir', 'rtl'); else document.documentElement.removeAttribute('dir');
        }
      }, { scale, dir });
    }
    this.scale = scale; this.dir = dir;
    await this.settle();
  }
  async settle() {
    await this.page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(() => r()))));
    await this.page.waitForTimeout(60);
  }
  async snapshot(opts = {}) {
    if (Object.keys(opts).length) await this.setView(opts);
    const ex = await this.page.evaluate(pageExtract);
    const ax = await this.cdp.send('Accessibility.getFullAXTree');
    const dom = await this.cdp.send('DOM.getDocument', { depth: -1, pierce: false });
    const backToId = new Map();
    (function walk(n) {
      if (n.attributes) { const a = n.attributes; for (let i = 0; i < a.length; i += 2) if (a[i] === 'data-pvc-id') backToId.set(n.backendNodeId, a[i + 1]); }
      for (const c of n.children || []) walk(c);
    })(dom.root);
    const axById = new Map();
    for (const n of ax.nodes) {
      if (n.backendDOMNodeId == null) continue;
      const id = backToId.get(n.backendDOMNodeId);
      if (id) axById.set(id, n);
    }
    for (const node of ex.nodes) {
      const a = axById.get(node.id);
      if (a) {
        const rl = (a.role?.value || '').toLowerCase();
        const props = axProps(a);
        node.ax_ignored = !!a.ignored;
        if (!a.ignored && !NON_ROLES.has(rl)) node.role = ROLE_MAP[rl] ?? rl;
        if (a.ignored && (node.visual || !node.text)) node.a11y_hidden = true;
        if (!a.ignored) {
          node.name = a.name?.value !== undefined && String(a.name.value).trim() !== '' ? String(a.name.value).replace(/\s+/g, ' ').trim() : null;
          node.name_source = node.name ? nameSource(a) : 'none';
          if (node.role === 'heading' && props.level !== undefined) node.states.level = +props.level;
          for (const k of ['checked', 'pressed']) if (props[k] !== undefined) node.states[k] = props[k] === 'mixed' ? 'mixed' : (props[k] === true || props[k] === 'true');
          for (const k of ['disabled', 'expanded', 'selected', 'required', 'busy']) if (props[k] !== undefined) node.states[k] = props[k] === true || props[k] === 'true';
          if (props.invalid !== undefined) node.states.invalid = !(props.invalid === false || props.invalid === 'false');
          if (props.live) node.states.live = props.live;
          if (a.value?.value !== undefined && a.value.value !== '') node.states.value = a.value.value;
          if (props.valuemin !== undefined) node.states.min = props.valuemin;
          if (props.valuemax !== undefined) node.states.max = props.valuemax;
        }
      } else if (['input', 'textarea', 'select'].includes(node.kind)) {
        // not in AX tree (hidden) -> leave role null
      }
      if (node.value !== undefined) node.states.value = node.value;
      if (node.role && ['text-field', 'text-area', 'search-field', 'email-field', 'stepper'].includes(node.role) && node.value === undefined) { /* contenteditable */ }
      // normalise tag-derived roles the AX tree reports with different names
      if (node.role === 'text-field' && node.kind === 'textarea') node.role = 'text-area';
      if (node.role === 'text-field' && node.attrs?.type === 'email') node.role = 'email-field';
      if (node.role === 'text-field' && node.attrs?.type === 'password') node.role = 'password-field';
      if (node.role === 'text-field' && node.attrs?.type === 'search') node.role = 'search-field';
      if (node.kind === 'summary' && node.role === 'disclosure-triangle') node.role = 'button';
      if (node.states.disabled === undefined && node.kind && ['button', 'input', 'select', 'textarea'].includes(node.kind)) { /* from AX */ }
    }
    // a11y_hidden for nodes absent from the AX tree: inherit from aria-hidden chain only
    return {
      meta: { viewport: [...this.viewport], text_scale: this.scale, direction: this.dir, doc_size: ex.doc, scroll_overflow_x: ex.overflowX, focused: ex.focused, locale: ex.lang || 'en' },
      nodes: ex.nodes,
    };
  }
  loc(id) { return this.page.locator(`[data-pvc-id="${id}"]`).first(); }
  async click(id) {
    await this.loc(id).click({ timeout: 2500 });
    await this.settle();
  }
  async type(id, text, { replace = false } = {}) {
    const l = this.loc(id);
    if (replace) await l.fill(text, { timeout: 2500 });
    else { await l.click({ timeout: 2500 }); await this.page.keyboard.type(text, { delay: 0 }); }
    await this.settle();
  }
  async typeFocused(text) { await this.page.keyboard.type(text, { delay: 0 }); await this.settle(); }
  async key(k) { await this.page.keyboard.press(k.replace('Space', 'Space')); await this.settle(); return this.focused(); }
  async focused() { return this.page.evaluate(() => { const a = document.activeElement; return a && a !== document.body && a !== document.documentElement && a.getAttribute ? a.getAttribute('data-pvc-id') : null; }); }
  async blurAll() { await this.page.evaluate(() => { if (document.activeElement && document.activeElement.blur) document.activeElement.blur(); const b = document.body; const had = b.hasAttribute('tabindex'); if (!had) b.setAttribute('tabindex', '-1'); b.focus({ preventScroll: true }); if (!had) b.removeAttribute('tabindex'); window.scrollTo(0, 0); }); }
  async close() { await this.page.close(); }
}

/** Launch a browser that can only reach `origin`; returns {newProbe(), newSessionProbe(), close()} */
export async function launchBrowser(origin) {
  const browser = await chromium.launch({ headless: true });
  const blocked = [];
  let context = null;
  async function mkContext() {
    const c = await browser.newContext({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1, locale: 'en-US', reducedMotion: 'no-preference' });
    await c.route('**/*', route => {
      const u = route.request().url();
      if (u.startsWith(origin) || u.startsWith('data:') || u.startsWith('blob:') || u.startsWith('about:')) return route.continue();
      blocked.push(u); return route.abort();
    });
    await c.addInitScript(() => {
      const st = document.createElement('style');
      st.textContent = '*,*::before,*::after{transition:none !important;animation:none !important;scroll-behavior:auto !important;caret-color:transparent}';
      const add = () => { (document.head || document.documentElement).appendChild(st); };
      if (document.head) add(); else document.addEventListener('DOMContentLoaded', add);
    });
    return c;
  }
  async function newProbe({ sameContext = false, path = '/' } = {}) {
    if (!sameContext || !context) { if (context) await context.close(); context = await mkContext(); }
    const page = await context.newPage();
    const cdp = await context.newCDPSession(page);
    await cdp.send('Accessibility.enable'); await cdp.send('DOM.enable');
    const consoleErrors = [];
    page.on('pageerror', e => consoleErrors.push(String(e)));
    await page.goto(origin + path, { waitUntil: 'load' });
    await page.waitForTimeout(120);
    const p = new BrowserProbe(page, cdp);
    p.pageErrors = consoleErrors;
    await p.setView({});
    return p;
  }
  return { newProbe, blocked, close: async () => { if (context) await context.close(); await browser.close(); } };
}
