import { R, ok, bad, cannot, norm } from './_common.mjs';

const LM = {
  header: { role: 'banner', alt: /header|banner/i },
  nav: { role: 'navigation', alt: null },
  main: { role: 'main', alt: null },
  aside: { role: 'complementary', alt: /sidebar|aside|related|complementary/i },
  footer: { role: 'contentinfo', alt: /footer/i },
};
/** visible landmark nodes of a kind: ARIA role, or (accepted on both stacks) a `region` whose name matches */
export function landmarks(s, kind, { visibleOnly = true } = {}) {
  const d = LM[kind];
  return s.nodes.filter(n => (!visibleOnly || s.isVisible(n)) && (n.role === d.role || (d.alt && n.role === 'region' && n.name && d.alt.test(n.name))));
}
const beside = (s, a, main) => R.vOverlap(a.rect, main.rect) > 0.3 * Math.min(a.rect[3], main.rect[3]) && R.hOverlap(a.rect, main.rect) < 0.2 * Math.min(a.rect[2], main.rect[2]);

export default {
  id: 'L04', category: '1 Layout-heavy', baseline: 'B1',
  intent: 'Holy-grail layout: header, left nav, main content, right sidebar, footer; sidebars collapse on tablets',
  fixture: ['The left navigation has at least three links, the right sidebar has a short list of related links, and the main content is short (one heading and two or three sentences) so the page is shorter than the screen. No external resources are available.'],
  conventions: [
    'landmarks: header = role banner (or region named header/banner); left nav = navigation; main = main; right sidebar = complementary (or region named sidebar/aside/related/complementary); footer = contentinfo (or region named footer). [G4 §8.1 has no banner/contentinfo/complementary role: see PVC_CHECKER_NEEDS §6.1]',
    'a nav "toggle" = a visible button whose name matches /menu|nav|sidebar|toggle|hamburger|open|☰|≡/i, or any visible button with an `expanded` state',
  ],
  items: [
    {
      id: 'L04.1', kind: 'A', g2: 'Five regions with landmark roles',
      op: ['At 1280×800: each of header, nav, main, sidebar, footer found as a visible landmark (conventions).',
        'Arrangement: header bottom ≤ top of nav/main/sidebar (+2 px); footer top ≥ bottom of nav/main/sidebar (−2 px); nav right edge ≤ main left edge + 2; main right edge ≤ sidebar left edge + 2; nav, main, sidebar overlap vertically.'],
      feedback: 'Not met: the page must have five regions with landmark roles (header, left navigation, main content, right sidebar, footer) arranged in the holy-grail layout.',
      async run(ctx) {
        const s = await ctx.at(1280);
        const f = {}; for (const k of Object.keys(LM)) { f[k] = landmarks(s, k)[0]; if (!f[k]) return bad(`missing landmark: ${k}`); }
        const mid = [f.nav, f.main, f.aside];
        if (!(R.b(f.header.rect) <= Math.min(...mid.map(n => n.rect[1])) + 2)) return bad('header not above the middle row');
        if (!(f.footer.rect[1] >= Math.max(...mid.map(n => R.b(n.rect))) - 2)) return bad('footer not below the middle row');
        if (!R.leftOf(f.nav.rect, f.main.rect) || !R.leftOf(f.main.rect, f.aside.rect)) return bad('nav | main | sidebar not left-to-right');
        if (R.vOverlap(f.nav.rect, f.main.rect) <= 0 || R.vOverlap(f.main.rect, f.aside.rect) <= 0) return bad('nav/main/sidebar not in one band');
        return ok();
      },
    },
    {
      id: 'L04.2', kind: 'A', trace: true, g2: 'Sidebars hidden ≤ 768 px with a way to reach the nav',
      op: ['At 768, 480 and 320 px, with main visible: the right sidebar is collapsed = not visible, or visible but not beside main (not in the same band side-by-side).',
        'The left nav is not shown as a side column beside main. Either it is visible and stacked (not beside main), or it is hidden (not visible) AND a visible nav toggle (conventions) exists such that activating it (pointer click / keyboard Enter) makes a visible `navigation` landmark with ≥ 2 visible links appear.'],
      feedback: 'Not met: at 768 px and narrower the sidebars must collapse, and the navigation must remain reachable.',
      async run(ctx) {
        for (const w of [768, 480, 320]) {
          let s = await ctx.at(w);
          const main = landmarks(s, 'main')[0];
          if (!main) return cannot(`${w}px: main landmark not visible`);
          const asides = landmarks(s, 'aside');
          if (asides.some(a => beside(s, a, main))) return bad(`${w}px: right sidebar still beside main`);
          const navs = landmarks(s, 'nav');
          if (navs.some(n => beside(s, n, main))) return bad(`${w}px: nav still a side column`);
          if (navs.length === 0) {
            const toggles = s.byRole('button').filter(b => (b.name && /menu|nav|sidebar|toggle|hamburger|open|☰|≡/i.test(b.name)) || b.states.expanded !== undefined);
            if (!toggles.length) return bad(`${w}px: nav hidden and no toggle control`);
            let revealed = false;
            for (const t of toggles) {
              await ctx.activate(t);
              s = await ctx.now();
              const nv = landmarks(s, 'nav')[0];
              if (nv && s.descendants(nv).filter(d => d.role === 'link' && s.isVisible(d)).length >= 2) { revealed = true; break; }
              await ctx.reload(); s = await ctx.at(w);
            }
            if (!revealed) return bad(`${w}px: activating the toggle does not reveal the navigation`);
            await ctx.reload();
          }
        }
        return ok();
      },
    },
    {
      id: 'L04.3', kind: 'A', g2: 'Footer at the bottom of short pages',
      op: ['Viewport 1280×1600 (taller than the content). footer = visible footer landmark. Pass iff footer bottom ≥ document height − 16 px (document height = max(content height, viewport height)), i.e. the footer sits at the bottom of the viewport on a short page.'],
      feedback: 'Not met: on a short page the footer must sit at the bottom of the screen.',
      async run(ctx) {
        const s = await ctx.at(1280, { h: 1600 });
        const f = landmarks(s, 'footer')[0];
        if (!f) return cannot('no footer landmark');
        const docH = s.meta.doc_size[1];
        if (R.b(f.rect) < docH - 16) return bad(`footer bottom ${Math.round(R.b(f.rect))} vs document ${docH}`);
        return ok();
      },
    },
  ],
};
