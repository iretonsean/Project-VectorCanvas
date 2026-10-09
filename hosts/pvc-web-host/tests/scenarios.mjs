// Scripted-input scenarios (brief item 5). Each exports {specimen, run(s, ctx)}.
// `s` is the object returned by lib.open(). dev-measure imports these for measured runs.
import path from 'node:path';
import { sleep, rects, frames, meta, state, centre, settle, writeJson, axTree, axCompact, axFind, IME_LABEL } from './lib.mjs';

export const GAP_MS = 50; // inter-event gap, METHOD.md §2.1

async function typeLatin(s, text, gap = GAP_MS) {
  for (const ch of text) { await s.page.keyboard.type(ch); await sleep(gap); }
}

export const SCENARIOS = {
  smoke: {
    async run(s, { outDir }) {
      const m = await meta(s.page);
      const f = await frames(s.page);
      const r = await rects(s.page);
      await s.page.screenshot({ path: path.join(outDir, 'smoke.png') });
      const gpu = await s.page.evaluate(() => typeof navigator.gpu);
      const pass = gpu === 'undefined' && m.webgl2 === true && f.length >= 1;
      return { pass, summary: { navigator_gpu: gpu, renderer: m.unmasked_renderer, frames: f.length, rects: r, perf_caveat_bypass: m.perf_caveat_bypass }, meta: m, frames: f };
    },
  },

  // A5 item 3: Chromium's accessibility tree (CDP Accessibility.getFullAXTree) shows every F01 interactive
  // element with correct role/name/state; checkbox toggle and an error appearing are reflected.
  a11y: {
    async run(s, { outDir }) {
      const p = s.page;
      const R = await rects(p);
      const snap = async (tag) => {
        await settle(p);
        const compact = axCompact(await axTree(s.cdp));
        writeJson(path.join(outDir, `ax_${tag}.json`), compact);
        return compact;
      };
      const a0 = await snap('0_initial');
      await p.mouse.click(...centre(R.checkbox));                       // checkbox toggle
      const a1 = await snap('1_checkbox_checked');
      await p.mouse.click(...centre(R.email));                          // an error appearing
      await settle(p);
      await p.keyboard.type('not-an-email', { delay: 15 });
      const a2 = await snap('2_email_invalid_error');
      for (let i = 0; i < 12; i++) await p.keyboard.press('Backspace');
      await p.keyboard.type('ada@example.com', { delay: 15 });
      await p.mouse.click(...centre(R.name)); await p.keyboard.type('Ada Lovelace', { delay: 15 });
      await p.mouse.click(...centre(R.password)); await p.keyboard.type('abc', { delay: 15 });
      const a3 = await snap('3_password_short_error');
      await p.keyboard.type('defghij', { delay: 15 });                  // 10 chars: valid
      const a4 = await snap('4_valid_submit_enabled');
      await p.mouse.click(...centre(R.toggle));                         // show/hide toggle
      const a5 = await snap('5_password_shown');
      await p.mouse.click(...centre(R.submit));                         // submit -> done message
      const a6 = await snap('6_submitted');
      const st = await state(p);
      const checks = [];
      const chk = (id, ok, detail) => checks.push({ id, ok: !!ok, detail });
      const one = (c, role, name) => axFind(c, role, name)[0];
      const truthy = (v) => v === true || v === 'true';
      const interactive = [['textbox', 'Full name'], ['textbox', 'Email'], ['textbox', 'Password'], ['button', 'Show password'], ['checkbox', 'I accept the terms of service'], ['button', 'Sign up']];
      for (const [role, name] of interactive) chk(`present: ${role} "${name}"`, !!one(a0, role, name), one(a0, role, name));
      chk('heading level 1 "Create account"', one(a0, 'heading', 'Create account')?.props.level === 1, one(a0, 'heading'));
      chk('required on the three fields', ['Full name', 'Email', 'Password'].every((n) => truthy(one(a0, 'textbox', n)?.props.required)), null);
      chk('checkbox unchecked initially', one(a0, 'checkbox')?.props.checked === 'false', one(a0, 'checkbox')?.props);
      chk('checkbox checked after toggle', one(a1, 'checkbox')?.props.checked === 'true', one(a1, 'checkbox')?.props);
      chk('submit disabled initially', truthy(one(a0, 'button', 'Sign up')?.props.disabled), one(a0, 'button', 'Sign up')?.props);
      chk('live regions exist from the first tree (status polite)', a0.filter((n) => n.role === 'status' && n.props.live === 'polite').length >= 3, a0.filter((n) => n.role === 'status').length);
      const errText = 'Enter a valid email address';
      chk('no email error text initially', !JSON.stringify(a0).includes(errText), null);
      chk('email error text appears (live region) after invalid email', JSON.stringify(a2).includes(errText), a2.filter((n) => n.role === 'status'));
      chk('email field invalid + describedby/errormessage', truthy(one(a2, 'textbox', 'Email')?.props.invalid), one(a2, 'textbox', 'Email')?.props);
      chk('email error gone when valid', !JSON.stringify(a4).includes(errText), null);
      chk('password error text after 3 chars', JSON.stringify(a3).includes('Password must be at least 8 characters'), null);
      chk('submit enabled when valid', !truthy(one(a4, 'button', 'Sign up')?.props.disabled), one(a4, 'button', 'Sign up')?.props);
      chk('password masked in AX value before show', (one(a4, 'textbox', 'Password')?.value || '').length > 0 && !(one(a4, 'textbox', 'Password')?.value || '').includes('abc'), one(a4, 'textbox', 'Password')?.value);
      chk('show/hide toggle pressed state flips', one(a4, 'button', 'Show password')?.props.pressed === 'false' && one(a5, 'button', 'Show password')?.props.pressed === 'true', [one(a4, 'button', 'Show password')?.props, one(a5, 'button', 'Show password')?.props]);
      chk('submitted: "Account created" announced', JSON.stringify(a6).includes('Account created'), a6.filter((n) => n.role === 'status'));
      return { pass: checks.every((c) => c.ok), summary: { checks: checks.map((c) => `${c.ok ? 'PASS' : 'FAIL'} ${c.id}`), state: st.raw }, checks };
    },
  },
  // Web path of the load-time signature check: a one-byte-flipped artifact must be refused, nothing rendered.
  tamper: {
    specimen: 'F01',
    open: { artifact: 'F01.tampered', allowError: true },
    async run(s, { outDir }) {
      const m = await meta(s.page);
      const f = await frames(s.page);
      const pass = !!m.error && /load_signed refused|Runtime::from_signed/.test(m.error) && f.length === 0 && m.webgl2 !== true;
      return { pass, summary: { error: m.error, frames_presented: f.length, artifact: m.artifact } };
    },
  },
  // Mirror-cost / frame-time preview (Q3, Q4). Same scripts as METHOD.md §2.1; dev-measure's harness is authoritative.
  perf: {
    async run(s0, { outDir, opt }) {
      const { open } = await import('./lib.mjs');
      await s0.close();
      const reps = Number(opt('reps', 3));
      const CSV_HEAD = 'frame_idx,t_ns,scenario,event,update_ms,layout_ms,scene_ms,a11y_ms,mirror_ms,render_ms,total_ms,completion_ms';
      const cells = [];
      for (const variant of ['raf', 'sync']) for (const mirror of ['on', 'off']) for (const sc of Object.keys(PERF_DRIVERS)) {
        const pooled = [];
        for (let rep = 0; rep < reps; rep++) {
          const s = await open({ scenario: sc, variant, mirror, specimen: PERF_DRIVERS[sc].specimen, locale: PERF_DRIVERS[sc].locale || 'en' });
          try {
            await PERF_DRIVERS[sc].drive(s);
            await settle(s.page, 300);
            const rows = await frames(s.page);
            const dir = path.join(outDir, `${variant}_mirror-${mirror}`, sc);
            fs.mkdirSync(dir, { recursive: true });
            const csv = [CSV_HEAD, ...rows.map((r) => CSV_HEAD.split(',').map((k) => (r[k] == null ? '' : typeof r[k] === 'number' && k.endsWith('_ms') ? r[k].toFixed(3) : r[k])).join(','))].join('\n');
            fs.writeFileSync(path.join(dir, `trace_rep${rep}.csv`), csv + '\n');
            const m = await meta(s.page);
            fs.writeFileSync(path.join(dir, `meta_rep${rep}.json`), JSON.stringify({ ...m, loadavg_at_start: s.loadavg }, null, 1));
            const measured = rows.filter((r) => PERF_DRIVERS[sc].classes.includes(r.event)).slice(10); // METHOD §10: discard first 10 measured-class frames
            pooled.push(...measured);
          } finally { await s.close(); }
        }
        cells.push({ variant, mirror, scenario: sc, n: pooled.length, ...cellStats(pooled) });
      }
      writeJson(path.join(outDir, 'perf_summary.json'), { reps, cells });
      return { pass: true, summary: cells };
    },
  },
  // Golden-image states for dev-render A7. Pixels come from readPixels (not a screenshot) + the display-list fingerprint.
  golden: {
    async run(s0, { outDir }) {
      const { open } = await import('./lib.mjs');
      await s0.close();
      const out = {};
      const grab = async (s, tag) => {
        const g = await s.page.evaluate(() => window.__pvcGoldenCapture());
        fs.mkdirSync(outDir, { recursive: true });
        fs.writeFileSync(path.join(outDir, `${tag}.png`), Buffer.from(g.rgba_png_base64, 'base64'));
        out[tag] = { dl_fingerprint: g.dl_fingerprint, width: g.width, height: g.height, png: `${tag}.png`, renderer: (await meta(s.page)).unmasked_renderer };
      };
      let s = await open({ specimen: 'L01', scenario: 'golden' });
      await settle(s.page); await grab(s, 'L01_idle'); await s.close();
      s = await open({ specimen: 'F01', scenario: 'golden' });
      await settle(s.page); await grab(s, 'F01_empty');
      await s.page.evaluate(() => { __pvc.focus('email'); __pvc.text('not-an-email'); });
      await settle(s.page); await grab(s, 'F01_error'); await s.close();
      s = await open({ specimen: 'F01', locale: 'zh-CN', scenario: 'golden' });
      await settle(s.page);
      await s.page.evaluate(() => { __pvc.focus('name'); __pvc.ime_preedit('nihao', 5, 5); });
      await settle(s.page); await grab(s, 'F01_zhCN_preedit'); await s.close();
      writeJson(path.join(outDir, 'golden.json'), out);
      return { pass: Object.values(out).every((v) => v.dl_fingerprint && v.width === 1366), summary: out };
    },
  },
  // Overlay <-> runtime selection sync, and no keyboard trap on Tab (WCAG 2.1.2).
  selection: {
    async run(s) {
      const p = s.page; const R = await rects(p);
      await p.mouse.click(...centre(R.name)); await settle(p);
      await p.keyboard.type('Ada Lovelace', { delay: 15 }); await settle(p);
      const sel = async () => (await state(p)).focused_input?.selection;
      const dom = () => p.evaluate(() => [document.activeElement.selectionStart, document.activeElement.selectionEnd]);
      const out = {};
      await p.keyboard.press('Control+A'); await settle(p); out.ctrlA = { runtime: await sel(), dom: await dom() };
      await p.keyboard.press('ArrowLeft'); await settle(p); out.left = { runtime: await sel(), dom: await dom() };
      await p.keyboard.press('Shift+ArrowLeft'); await settle(p); out.shiftLeft = { runtime: await sel(), dom: await dom() };
      await p.mouse.dblclick(R.name[0] + 90, R.name[1] + R.name[3] / 2); await settle(p); out.dblclick = { runtime: await sel(), dom: await dom() };
      // Tab out of the last stop must hand focus to the browser (not trap)
      for (let i = 0; i < 4; i++) { await p.keyboard.press('Tab'); await settle(p, 80); }
      out.afterTabs = await p.evaluate(() => (document.activeElement.getAttribute('aria-label') || document.activeElement.tagName));
      await p.keyboard.press('Tab'); await settle(p, 80);
      out.afterOneMoreTab = await p.evaluate(() => (document.activeElement.getAttribute('aria-label') || document.activeElement.tagName));
      const same = (x) => x.runtime && x.dom && x.runtime[0] === x.dom[0] && x.runtime[1] === x.dom[1];
      return { pass: same(out.ctrlA) && same(out.left) && same(out.shiftLeft) && same(out.dblclick), summary: out };
    },
  },
  'ptr-move': {
    specimen: 'L01',
    async run(s, { outDir }) {
      const p = s.page;
      const R = await rects(p);
      if (!R || !R.connect_button || !R.bio_text) return { pass: false, summary: { error: 'L01 rects (connect_button, bio_text) not published; needs the real runtime + L01', rects: R } };
      const inn = centre(R.connect_button), out = centre(R.bio_text);
      await p.mouse.move(...inn); await settle(p);
      const f0 = (await frames(p)).length;
      for (let i = 0; i < 200; i++) { await p.mouse.move(...(i % 2 === 0 ? out : inn)); await sleep(GAP_MS); }
      await settle(p);
      const f1 = (await frames(p)).length;
      return { pass: true, summary: { events_sent: 200, frames: f1 - f0, frame_per_event: (f1 - f0) / 200 } };
    },
  },
  tab: {
    async run(s, { outDir }) {
      const p = s.page;
      const R = await rects(p);
      await p.mouse.click(20, 20); await settle(p); // click on empty background (setup)
      const f0 = (await frames(p)).length;
      const order = [];
      for (let i = 0; i < 60; i++) {
        await p.keyboard.press('Tab'); await sleep(GAP_MS);
        order.push(await p.evaluate(() => { const a = document.activeElement; return a ? (a.getAttribute('aria-label') || a.id || a.tagName) : null; }));
      }
      await settle(p);
      const f1 = (await frames(p)).length;
      const st = await state(p);
      return { pass: new Set(order).size >= 5, summary: { events_sent: 60, frames: f1 - f0, frame_per_event: (f1 - f0) / 60, first_cycle: order.slice(0, 7), distinct_stops: [...new Set(order)] }, order, state: st };
    },
  },
  'type-latin': {
    async run(s, { outDir }) {
      const p = s.page;
      const R = await rects(p);
      await p.mouse.click(...centre(R.name)); await settle(p);
      const text = 'Ada Lovelace Byron 1';
      const f0 = (await frames(p)).length;
      let ok = 0;
      for (let round = 0; round < 5; round++) {
        await typeLatin(s, text);
        await settle(p);
        const st = await state(p);
        if (st.name === text) ok++; else console.error('mismatch', JSON.stringify(st.name));
        await p.keyboard.press('Control+A'); await p.keyboard.press('Delete');
        // runtime may not implement select-all/Delete: clear with backspaces so each round starts empty
        for (let i = 0; i < 30; i++) await p.keyboard.press('Backspace');
        await settle(p);
      }
      const f1 = (await frames(p)).length;
      return { pass: ok === 5, summary: { rounds_with_exact_text: ok, of: 5, events_sent: 100, frames: f1 - f0, frame_per_event: (f1 - f0) / 100 } };
    },
  },
  'toggle-show': {
    async run(s) {
      const p = s.page; const R = await rects(p);
      await p.mouse.move(...centre(R.toggle)); await settle(p);
      const f0 = (await frames(p)).length;
      const flips = [];
      for (let i = 0; i < 100; i++) { await p.mouse.down(); await sleep(GAP_MS / 2); await p.mouse.up(); await sleep(GAP_MS / 2); if (i < 3) flips.push((await state(p)).show_pw); }
      await settle(p);
      const f1 = (await frames(p)).length; const st = await state(p);
      return { pass: st.show_pw === false, summary: { clicks: 100, final_show: st.show_pw, first_flips: flips, frames: f1 - f0 } };
    },
  },
  checkbox: {
    async run(s) {
      const p = s.page; const R = await rects(p);
      await p.mouse.move(...centre(R.checkbox)); await settle(p);
      const f0 = (await frames(p)).length;
      for (let i = 0; i < 100; i++) { await p.mouse.down(); await sleep(GAP_MS / 2); await p.mouse.up(); await sleep(GAP_MS / 2); }
      await settle(p);
      const f1 = (await frames(p)).length; const st = await state(p);
      return { pass: st.terms === false, summary: { clicks: 100, final_terms: st.terms, frames: f1 - f0 } };
    },
  },
  // METHOD.md §6: 1 s settle, then 5 s with no input. Pass value is exactly 0 frames (two witnesses).
  idle: {
    async run(s) {
      const p = s.page;
      await p.evaluate(() => {
        window.__glCalls = [];
        for (const fn of ['drawArrays', 'drawElements', 'drawArraysInstanced', 'drawElementsInstanced', 'clear', 'clearBufferfv', 'readPixels']) {
          const orig = WebGL2RenderingContext.prototype[fn];
          WebGL2RenderingContext.prototype[fn] = function (...a) { window.__glCalls.push(performance.now()); return orig.apply(this, a); };
        }
      });
      await sleep(1000);
      const f0 = (await frames(p)).length; const g0 = await p.evaluate(() => window.__glCalls.length);
      await sleep(5000);
      const f1 = (await frames(p)).length; const g1 = await p.evaluate(() => window.__glCalls.length);
      // idle-focused (reported, not part of the pass): name field focused
      const R = await rects(p);
      await p.mouse.click(...centre(R.name)); await settle(p, 300);
      const h0 = (await frames(p)).length; const gh0 = await p.evaluate(() => window.__glCalls.length);
      await sleep(5000);
      const h1 = (await frames(p)).length; const gh1 = await p.evaluate(() => window.__glCalls.length);
      return { pass: f1 - f0 === 0 && g1 - g0 === 0, summary: { idle_initial: { frames_in_5s: f1 - f0, gl_draw_calls_in_5s: g1 - g0 }, idle_focused_info_only: { frames_in_5s: h1 - h0, gl_draw_calls_in_5s: gh1 - gh0 } } };
    },
  },
  // A5 item 4. IME driven through Chromium's composition pipeline via CDP. IME_LABEL must accompany every claim.
  ime: {
    locale: 'zh-CN',
    async run(s, { outDir }) {
      const p = s.page, cdp = s.cdp;
      const R = await rects(p);
      await p.mouse.click(...centre(R.name)); await settle(p);
      const events = [];
      await p.evaluate(() => {
        window.__imeLog = [];
        const el = document.activeElement;
        for (const t of ['compositionstart', 'compositionupdate', 'compositionend', 'beforeinput', 'input', 'keydown']) el.addEventListener(t, (e) => window.__imeLog.push({ t, data: e.data ?? null, inputType: e.inputType ?? null, composing: e.isComposing ?? null, value: el.value }), true);
      });
      const clip = { x: R.name[0] - 4, y: R.name[1] - 4, width: R.name[2] + 8, height: R.name[3] + 8 };
      await p.screenshot({ path: path.join(outDir, 'field_before.png'), clip });
      const steps = [];
      const shots = [];
      const before = await p.screenshot({ clip });
      for (const pre of ['n', 'ni', 'nih', 'niha', 'nihao']) {
        await cdp.send('Input.imeSetComposition', { text: pre, selectionStart: pre.length, selectionEnd: pre.length });
        await settle(p);
        const st = await state(p);
        steps.push({ preedit_sent: pre, runtime_composing: st.focused_input?.composing, runtime_value: st.focused_input?.value, runtime_name: st.name, dom_value: await p.evaluate(() => document.activeElement.value) });
        shots.push(await p.screenshot({ clip }));
        if (pre === 'nihao') await p.screenshot({ path: path.join(outDir, 'field_preedit_nihao.png'), clip });
      }
      await p.screenshot({ path: path.join(outDir, 'page_preedit_nihao.png') });
      await cdp.send('Input.insertText', { text: '你好' });
      await settle(p);
      const st = await state(p);
      await p.screenshot({ path: path.join(outDir, 'field_committed.png'), clip });
      const ax = axCompact(await axTree(cdp));
      writeJson(path.join(outDir, 'ax_after_commit.json'), ax);
      const axName = ax.find((n) => n.role === 'textbox' && n.name === '姓名');
      const log = await p.evaluate(() => window.__imeLog);
      writeJson(path.join(outDir, 'dom_event_log.json'), log);
      const ink = { diffPx: await diffPixels(s.context, before, shots[4]), stepsDiffer: (await Promise.all([1, 2, 3, 4].map((i) => diffPixels(s.context, shots[i - 1], shots[i])))).every((n) => n > 0) };
      const checks = [
        ['runtime in composing state at every preedit step, committed value still empty', steps.every((x) => x.runtime_composing === true && x.runtime_name === '')],
        ['preedit drawn on the canvas (pixel diff of field region, before vs nihao)', ink.diffPx > 50],
        ['preedit text differs between steps (canvas redrawn each step)', ink.stepsDiffer],
        ['commit landed in runtime state', st.name === '你好'],
        ['preedit cleared after commit', st.focused_input?.composing === false],
        ['commit visible in AX tree (textbox "姓名" value)', axName && axName.value === '你好'],
      ];
      return { pass: checks.every((c) => c[1]), label: IME_LABEL, summary: { label: IME_LABEL, ink, checks: checks.map((c) => `${c[1] ? 'PASS' : 'FAIL'} ${c[0]}`), steps, final_state: st, ax_value: axName && axName.value, dom_events: log.map((e) => e.t + ':' + (e.data ?? e.inputType ?? '')) } };
    },
  },

  // G6 risk 5: overlay <input> vs canvas caret/text alignment at DSF 1.0 / 1.25 / 1.5 / 2.0.
  // Canvas side is pixel-measured (screenshot). Native-caret side is measured from DOM text metrics because
  // headless Chromium paints no native caret in screenshots (proved by caret_paint_check below).
  // Overlay text side is pixel-measured with the canvas hidden and the <input> text drawn opaque magenta.
  dsf: {
    async run(s0, { outDir }) {
      const { open } = await import('./lib.mjs');
      await s0.close();
      const rows = [];
      let caretPaintCheck;
      for (const [tr, dsf] of [1, 1.25, 1.5, 2].flatMap((d) => [['geometricPrecision', d], ['auto', d]])) {
        const s = await open({ ...s0.o, dsf, scenario: 'dsf' });
        const p = s.page;
        try {
          const R = await rects(p);
          await p.evaluate((t) => document.querySelectorAll('.pvci').forEach((e) => { e.style.textRendering = t; }), tr);
          await p.mouse.click(...centre(R.name)); await settle(p);
          await p.keyboard.type('Ada Lovelace', { delay: 20 }); await settle(p);
          const info = await p.evaluate(() => {
            const el = document.activeElement; const r = el.getBoundingClientRect(); const cs = getComputedStyle(el);
            const id = el.id.replace('pvc-n', '');
            // native caret x from DOM text metrics: content-box left + advance of text before the selection end
            const span = document.createElement('span');
            span.style.cssText = `position:absolute;visibility:hidden;white-space:pre;font:${cs.font};letter-spacing:${cs.letterSpacing};text-rendering:${cs.textRendering};`;
            span.textContent = el.value.slice(0, el.selectionEnd);
            document.body.appendChild(span); const adv = span.getBoundingClientRect().width; span.remove();
            const nativeX = r.left + parseFloat(cs.borderLeftWidth) + parseFloat(cs.paddingLeft) + adv - el.scrollLeft;
            return { id, dpr: devicePixelRatio, canvas: [document.getElementById('pvc-canvas').width, document.getElementById('pvc-canvas').height], domRect: [r.x, r.y, r.width, r.height], font: cs.font, padding: cs.padding, value: el.value, caret: window.__pvc.caret_rect(id), native_caret_x_css: nativeX };
          });
          const caret = info.caret ? JSON.parse(info.caret) : null;
          const shotA = await p.screenshot({ type: 'png' }); // canvas as drawn
          await p.evaluate(() => { document.body.classList.add('pvc-debug'); document.getElementById('pvc-canvas').style.visibility = 'hidden'; document.querySelectorAll('.pvci').forEach((e) => { e.style.color = '#f0f'; e.style.outline = 'none'; }); document.getElementById('pvc-mirror').style.background = '#fff'; });
          // (text-rendering stays as set above)
          await sleep(100);
          const shotC = await p.screenshot({ type: 'png' }); // overlay text only, opaque magenta
          const meas = await analyse(s.context, shotA, shotC, caret, dsf, info.domRect);
          if (!caretPaintCheck) caretPaintCheck = await checkNativeCaretPaint(s.context);
          const native_caret_x_dev = info.native_caret_x_css * dsf;
          rows.push({
            dsf, overlay_text_rendering: tr, ...info, caret_engine_css: caret, ...meas, native_caret_x_dev_exact: native_caret_x_dev,
            caret_offset_dev_px: meas.canvas_caret_x_dev != null ? native_caret_x_dev - (meas.canvas_caret_x_dev + 0.5) : null,
            engine_caret_vs_native_dev_px: caret ? native_caret_x_dev - caret.x0 * dsf : null,
          });
          await writeBuf(path.join(outDir, `dsf_${dsf}_${tr}_canvas.png`), shotA);
          await writeBuf(path.join(outDir, `dsf_${dsf}_${tr}_overlay_only.png`), shotC);
        } finally { await s.close(); }
      }
      writeJson(path.join(outDir, 'dsf_table.json'), { caretPaintCheck, rows });
      return { pass: rows.every((r) => r.canvas_caret_x_dev != null), caretPaintCheck, summary: rows.map((r) => ({ dsf: r.dsf, overlay_text_rendering: r.overlay_text_rendering, engine_caret_x_dev: r.caret_engine_css ? +(r.caret_engine_css.x0 * r.dsf).toFixed(2) : null, native_caret_x_dev: +r.native_caret_x_dev_exact.toFixed(2), native_minus_engine_caret_dev_px: r.engine_caret_vs_native_dev_px == null ? null : +r.engine_caret_vs_native_dev_px.toFixed(2), text_ink_left_offset_dev_px: r.ink_left_offset_dev_px, text_ink_right_offset_dev_px: r.ink_right_offset_dev_px })) };
    },
  },
};

import fs from 'node:fs';
async function writeBuf(f, b) { fs.mkdirSync(path.dirname(f), { recursive: true }); fs.writeFileSync(f, b); }

// Proves headless Chromium does not paint the native text caret into screenshots (so it cannot be pixel-measured).
async function checkNativeCaretPaint(context) {
  const pg = await context.newPage();
  await pg.setContent('<input id=i style="font:16px sans-serif;caret-color:#f0f;width:200px" value="Ada">');
  await pg.focus('#i'); await pg.keyboard.press('End');
  let withMagenta = 0; const N = 8;
  for (let k = 0; k < N; k++) {
    const shot = await pg.screenshot({ clip: { x: 0, y: 0, width: 220, height: 40 } });
    const n = await pg.evaluate(async (b64) => { const i = new Image(); await new Promise((ok) => { i.onload = ok; i.src = 'data:image/png;base64,' + b64; }); const c = document.createElement('canvas'); c.width = i.width; c.height = i.height; const g = c.getContext('2d'); g.drawImage(i, 0, 0); const d = g.getImageData(0, 0, c.width, c.height).data; let m = 0; for (let o = 0; o < d.length; o += 4) if (d[o] > 235 && d[o + 1] < 60 && d[o + 2] > 235) m++; return m; }, shot.toString('base64'));
    if (n > 0) withMagenta++;
    await sleep(130);
  }
  await pg.close();
  return { screenshots: N, with_native_caret_pixels: withMagenta, conclusion: withMagenta === 0 ? 'headless Chromium paints no native caret in screenshots; native caret x measured from DOM text metrics instead' : 'native caret is paintable' };
}

// Decode two PNG screenshots on a blank page and locate the canvas caret column and text-ink boxes by pixel colour.
async function analyse(context, shotA, shotC, caretCss, dsf, domRect) {
  const pg = await context.newPage();
  const res = await pg.evaluate(async ({ a, c, caret, dsf, domRect }) => {
    const load = (b64) => new Promise((ok) => { const i = new Image(); i.onload = () => ok(i); i.src = 'data:image/png;base64,' + b64; });
    const px = async (b64) => { const i = await load(b64); const cv = document.createElement('canvas'); cv.width = i.width; cv.height = i.height; const g = cv.getContext('2d'); g.drawImage(i, 0, 0); return { w: i.width, h: i.height, d: g.getImageData(0, 0, i.width, i.height).data }; };
    const A = await px(a), C = await px(c);
    const y0 = Math.floor(domRect[1] * dsf) + 2, y1 = Math.ceil((domRect[1] + domRect[3]) * dsf) - 2;
    const xs0 = Math.floor(domRect[0] * dsf), xs1 = Math.ceil((domRect[0] + domRect[2]) * dsf);
    const cy0 = caret ? Math.floor(caret.y0 * dsf) : y0, cy1 = caret ? Math.ceil(caret.y1 * dsf) : y1;
    const colRuns = (I, test) => { const out = []; for (let x = xs0; x < xs1; x++) { let run = 0, best = 0; for (let y = cy0; y < cy1; y++) { const o = (y * I.w + x) * 4; if (test(I.d[o], I.d[o + 1], I.d[o + 2])) { run++; best = Math.max(best, run); } else run = 0; } if (best >= 0.6 * (cy1 - cy0)) out.push(x); } return out; };
    const bbox = (I, test) => { let l = 1e9, r = -1; for (let y = y0; y < y1; y++) for (let x = xs0; x < xs1; x++) { const o = (y * I.w + x) * 4; if (test(I.d[o], I.d[o + 1], I.d[o + 2])) { if (x < l) l = x; if (x > r) r = x; } } return r < 0 ? null : [l, r]; };
    const dark = (r, g, b) => r < 90 && g < 90 && b < 90;
    const cx = caret ? Math.floor(caret.x0 * dsf) : -99;
    const bboxNoCaret = (I, test) => { let l = 1e9, r = -1; for (let y = y0; y < y1; y++) for (let x = xs0; x < xs1; x++) { if (Math.abs(x - cx) <= 2) continue; const o = (y * I.w + x) * 4; if (test(I.d[o], I.d[o + 1], I.d[o + 2])) { if (x < l) l = x; if (x > r) r = x; } } return r < 0 ? null : [l, r]; };
    const mag = (r, g, b) => r > 170 && g < 110 && b > 170;
    return { canvas_dark_cols: colRuns(A, dark), canvas_ink: bboxNoCaret(A, dark), overlay_ink: bbox(C, mag) };
  }, { a: shotA.toString('base64'), c: shotC.toString('base64'), caret: caretCss, dsf, domRect });
  await pg.close();
  // the canvas caret is the right-most full-height dark column; text ink (box-per-char in the scaffold, glyphs with the real runtime) is left of it
  const canvas_caret_x_dev = res.canvas_dark_cols.length ? res.canvas_dark_cols[res.canvas_dark_cols.length - 1] : null;
  const ci = res.canvas_ink, oi = res.overlay_ink;
  return {
    canvas_caret_x_dev, canvas_ink_dev: ci, overlay_ink_dev: oi,
    ink_left_offset_dev_px: ci && oi ? oi[0] - ci[0] : null,
    ink_right_offset_dev_px: ci && oi ? oi[1] - ci[1] : null,
  };
}

// Number of pixels that differ between two PNG buffers of equal size (decoded in a scratch page).
async function diffPixels(context, a, b) {
  const pg = await context.newPage();
  const n = await pg.evaluate(async ({ a, b }) => {
    const px = async (b64) => { const i = new Image(); await new Promise((ok) => { i.onload = ok; i.src = 'data:image/png;base64,' + b64; }); const c = document.createElement('canvas'); c.width = i.width; c.height = i.height; const g = c.getContext('2d'); g.drawImage(i, 0, 0); return g.getImageData(0, 0, c.width, c.height).data; };
    const A = await px(a), B = await px(b); let n = 0;
    for (let o = 0; o < A.length; o += 4) if (Math.abs(A[o] - B[o]) + Math.abs(A[o + 1] - B[o + 1]) + Math.abs(A[o + 2] - B[o + 2]) > 30) n++;
    return n;
  }, { a: a.toString('base64'), b: b.toString('base64') });
  await pg.close();
  return n;
}

// ---- perf helpers
const pct = (a, p) => { if (!a.length) return null; const v = [...a].sort((x, y) => x - y); const k = (v.length - 1) * p / 100; const f = Math.floor(k), c = Math.ceil(k); return +(v[f] + (v[c] - v[f]) * (k - f)).toFixed(3); };
function cellStats(rows) {
  const col = (k) => rows.map((r) => r[k]).filter((x) => typeof x === 'number');
  const st = (a) => ({ p50: pct(a, 50), p95: pct(a, 95), p99: pct(a, 99), max: a.length ? +Math.max(...a).toFixed(3) : null });
  const mir = col('mirror_ms'), tot = col('total_ms');
  const ratio = rows.filter((r) => typeof r.mirror_ms === 'number' && r.total_ms > 0).map((r) => r.mirror_ms / r.total_ms);
  const sumM = mir.reduce((a, b) => a + b, 0), sumT = rows.filter((r) => typeof r.mirror_ms === 'number').reduce((a, r) => a + r.total_ms, 0);
  return {
    total_ms: st(tot), completion_ms: st(col('completion_ms')), mirror_ms: mir.length ? st(mir) : null,
    overlay_only_ms: col('overlay_only_ms').length ? st(col('overlay_only_ms')) : null,
    update_ms: st(col('update_ms')), layout_ms: st(col('layout_ms')), scene_ms: st(col('scene_ms')), a11y_ms: st(col('a11y_ms')), render_ms: st(col('render_ms')),
    mirror_share_of_total_sum: mir.length && sumT ? +(sumM / sumT).toFixed(4) : null,
    mirror_share_p50: ratio.length ? pct(ratio, 50) : null, mirror_share_p95: ratio.length ? pct(ratio, 95) : null,
  };
}
const PERF_DRIVERS = {
  tab: { specimen: 'F01', classes: ['key_tab'], async drive(s) { const p = s.page; await p.mouse.click(20, 20); await settle(p); for (let i = 0; i < 60; i++) { await p.keyboard.press('Tab'); await sleep(GAP_MS); } } },
  'type-latin': { specimen: 'F01', classes: ['key_char'], async drive(s) { const p = s.page; const R = await rects(p); await p.mouse.click(...centre(R.name)); await settle(p); for (let r = 0; r < 5; r++) { for (const ch of 'Ada Lovelace Byron 1') { await p.keyboard.type(ch); await sleep(GAP_MS); } await p.keyboard.press('Control+A'); await p.keyboard.press('Delete'); await settle(p); } } },
  'toggle-show': { specimen: 'F01', classes: ['ptr_down', 'ptr_up'], async drive(s) { const p = s.page; const R = await rects(p); await p.mouse.move(...centre(R.toggle)); await settle(p); for (let i = 0; i < 100; i++) { await p.mouse.down(); await sleep(GAP_MS / 2); await p.mouse.up(); await sleep(GAP_MS / 2); } } },
  checkbox: { specimen: 'F01', classes: ['ptr_down', 'ptr_up'], async drive(s) { const p = s.page; const R = await rects(p); await p.mouse.move(...centre(R.checkbox)); await settle(p); for (let i = 0; i < 100; i++) { await p.mouse.down(); await sleep(GAP_MS / 2); await p.mouse.up(); await sleep(GAP_MS / 2); } } },
  'ime-zh': { specimen: 'F01', locale: 'zh-CN', classes: ['ime_preedit', 'ime_commit'], async drive(s) { const p = s.page; const R = await rects(p); await p.mouse.click(...centre(R.name)); await settle(p); for (let r = 0; r < 20; r++) { for (const pre of ['n', 'ni', 'nih', 'niha', 'nihao']) { await s.cdp.send('Input.imeSetComposition', { text: pre, selectionStart: pre.length, selectionEnd: pre.length }); await sleep(GAP_MS); } await s.cdp.send('Input.insertText', { text: '你好' }); await sleep(GAP_MS); await p.keyboard.press('Backspace'); await p.keyboard.press('Backspace'); await settle(p, 120); } } },
};
