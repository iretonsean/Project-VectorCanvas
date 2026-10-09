// core/ctx.mjs — the per-item context handed to every checklist item. Stack-neutral: it only needs a "driver"
// that can open a fresh app instance and return a probe with the ops listed in PVC_CHECKER_NEEDS.md §2.
import { Snap, INTERACTIVE_ROLES } from './lib.mjs';

export class Ctx {
  /** driver: { fresh(): Promise<Probe>, newSession(): Promise<Probe> } ; mode: 'pointer' | 'keyboard' */
  constructor(driver, mode = 'pointer', pointerResults = {}) {
    this.driver = driver; this.mode = mode; this.p = null; this.view = { w: 1280, h: 800, scale: 1, dir: 'ltr' };
    this.pointerResults = pointerResults; this.log = [];
  }
  async init() { this.p = await this.driver.fresh(); this.view = { w: 1280, h: 800, scale: 1, dir: 'ltr' }; return this; }
  async reload() { await this.p.close?.(); await this.init(); }
  async close() { await this.p?.close?.(); }
  get dialogs() { return this.p.dialogs ?? []; }
  set dialogPolicy(v) { this.p.dialogPolicy = v; }
  /** snapshot at viewport width w (other view params persist unless overridden) */
  async at(w, extra = {}) {
    this.view = { ...this.view, w, ...extra };
    return new Snap(await this.p.snapshot(this.view));
  }
  /** snapshot with the current view settings */
  async now() { return new Snap(await this.p.snapshot(this.view)); }
  async newSession() {
    if (this.p.newSession) { await this.p.newSession(); this.view = { w: 1280, h: 800, scale: 1, dir: 'ltr' }; return; }
    await this.p.close?.(); this.p = await this.driver.newSession(); this.view = { w: 1280, h: 800, scale: 1, dir: 'ltr' }; }

  async click(n) { await this.p.click(n.id); }
  /** activate = press a button/link/checkbox/radio/tab the way the current mode does */
  async activate(n) {
    if (this.mode === 'pointer') return this.p.click(n.id);
    await this.kbFocus(n);
    await this.p.key(['checkbox', 'switch', 'radio'].includes(n.role) ? 'Space' : 'Enter');
  }
  /** type text into an input node. replace=true selects existing content first */
  async type(n, text, { replace = false } = {}) {
    if (this.mode === 'pointer') return this.p.type(n.id, text, { replace });
    await this.kbFocus(n);
    if (replace) { await this.p.key('Control+a'); }
    await this.p.typeFocused(text);
  }
  async key(k) { return this.p.key(k); }
  /** sequential focus order via real Tab presses from the start of the document */
  async focusOrder(max = 200) {
    await this.p.blurAll();
    const seen = []; const set = new Set(); let trapped = false;
    for (let i = 0; i < max; i++) {
      const f = await this.p.key('Tab');
      if (f == null) break;
      if (set.has(f)) break;
      set.add(f); seen.push(f);
      if (i === max - 1) trapped = true;
    }
    return { order: seen, trapped };
  }
  async tabTo(id) {
    await this.p.blurAll();
    for (let i = 0; i < 200; i++) { const f = await this.p.key('Tab'); if (f == null || f === id) return f === id; }
    return false;
  }
  /** move keyboard focus to node n using only Tab / arrow keys; throws if not reachable */
  async kbFocus(n) {
    if ((await this.p.focused()) === n.id) return;
    await this.p.blurAll();
    const order = [];
    for (let i = 0; i < 200; i++) {
      const f = await this.p.key('Tab');
      if (f == null || order.includes(f)) break;
      order.push(f);
      if (f === n.id) return;
    }
    // roving-focus groups (tabs, radios): reach a sibling then use arrows
    if (n.role === 'tab' || n.role === 'radio') {
      const snap = await this.now();
      const me = snap.get(n.id);
      let anc = me ? snap.parent(me) : null;
      while (anc && snap.nodes.filter(x => x.role === n.role && snap.contains(anc, x)).length < 2) anc = snap.parent(anc);
      const sibs = anc ? snap.nodes.filter(x => x.role === n.role && snap.contains(anc, x)) : [];
      const reach = sibs.find(s => order.includes(s.id));
      if (reach) {
        await this.p.blurAll();
        await this.tabTo(reach.id);
        for (const k of ['ArrowRight', 'ArrowDown', 'ArrowLeft', 'ArrowUp']) {
          for (let j = 0; j < sibs.length + 1; j++) {
            if ((await this.p.focused()) === n.id) return;
            await this.p.key(k);
          }
          if ((await this.p.focused()) === n.id) return;
          // return to start of the group for the next arrow direction
          await this.p.blurAll();
          await this.tabTo(reach.id);
        }
      }
    }
    throw new Error(`not reachable by keyboard: ${n.role} "${n.name ?? n.text ?? ''}"`);
  }
}
