// tools/measure/web_run.mjs — one repetition of one (specimen, scenario) on path chromium141-webgl2-swiftshader.
// usage: node web_run.mjs --www DIR --specimen L01|F01 --scenario S --variant raf|sync --mirror on|off --out DIR --rep K [--cmd "..."]
// Implements METHOD.md §2.1 (scripts), §2.2 (web), §6 (idle), §7 (first frame), §8 (RSS), §9 (noise probe).
// Writes <out>/trace_rep<K>.csv and <out>/runmeta_rep<K>.json. Must be run under `flock -x CPU_LOCK` by run.sh.
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import crypto from 'node:crypto';
import { createRequire } from 'node:module';
const require = createRequire('/opt/node22/lib/node_modules/');
const { chromium } = require('playwright');

const argv = process.argv.slice(2);
const opt = (k, d) => { const i = argv.indexOf('--' + k); return i >= 0 ? argv[i + 1] : d; };
const WWW = path.resolve(opt('www'));
const specimen = opt('specimen'), scenario = opt('scenario'), variant = opt('variant', 'raf'), mirror = opt('mirror', 'on');
const outDir = path.resolve(opt('out')); const rep = opt('rep', '1'); const cmd = opt('cmd', argv.join(' '));
const GAP = 50;
fs.mkdirSync(outDir, { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const centre = (r) => [r[0] + r[2] / 2, r[1] + r[3] / 2];

// ---------- noise probe (METHOD §9) ----------
function readStat() { const l = fs.readFileSync('/proc/stat', 'utf8').split('\n')[0].trim().split(/\s+/).slice(1).map(Number); return { idle: l[3] + (l[4] || 0), total: l.reduce((a, b) => a + b, 0) }; }
async function idleProbe(ms = 3000) { const a = readStat(); await sleep(ms); const b = readStat(); return +(100 * (b.idle - a.idle) / (b.total - a.total)).toFixed(1); }
let noisy = false; let probe = await idleProbe();
for (let i = 0; i < 6 && probe < 90; i++) { await sleep(20000); probe = await idleProbe(); }
if (probe < 90) noisy = true;
const loadavg = fs.readFileSync('/proc/loadavg', 'utf8').trim();

// ---------- static server ----------
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.otf': 'font/otf', '.ttf': 'font/ttf' };
const srv = http.createServer((req, res) => {
  const u = new URL(req.url, 'http://x'); let p = path.normalize(path.join(WWW, decodeURIComponent(u.pathname)));
  if (!p.startsWith(WWW)) { res.writeHead(403); return res.end(); }
  if (fs.existsSync(p) && fs.statSync(p).isDirectory()) p = path.join(p, 'index.html');
  if (!fs.existsSync(p)) { res.writeHead(404); return res.end('nf'); }
  res.writeHead(200, { 'content-type': MIME[path.extname(p)] || 'application/octet-stream', 'cache-control': 'no-store' });
  fs.createReadStream(p).pipe(res);
});
await new Promise((r) => srv.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${srv.address().port}`;

// ---------- artifacts hashes ----------
const sha = (f) => crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
const artifacts = {};
for (const sub of ['pkg', 'artifacts', 'fonts']) { const d = path.join(WWW, sub); if (fs.existsSync(d)) for (const f of fs.readdirSync(d)) if (fs.statSync(path.join(d, f)).isFile() && !f.startsWith('SHA256')) artifacts[`${sub}/${f}`] = sha(path.join(d, f)); }
artifacts['index.html'] = sha(path.join(WWW, 'index.html')); artifacts['boot.js'] = sha(path.join(WWW, 'boot.js'));

// ---------- launch ----------
const HIDE_GPU = () => { try { Object.defineProperty(Navigator.prototype, 'gpu', { get: () => undefined, configurable: true }); } catch (e) {} };
const GL_COUNT = () => { window.__glCalls = []; const wrap = (C) => { for (const fn of ['drawArrays', 'drawElements', 'drawArraysInstanced', 'drawElementsInstanced', 'clear', 'clearBufferfv', 'readPixels']) { const o = C.prototype[fn]; if (!o) continue; C.prototype[fn] = function (...a) { window.__glCalls.push(performance.now()); return o.apply(this, a); }; } }; wrap(WebGL2RenderingContext); if (typeof WebGLRenderingContext !== 'undefined') wrap(WebGLRenderingContext); };

const locale = scenario === 'ime-zh' ? 'zh-CN' : 'en';
const tLaunch0 = Date.now();
const browser = await chromium.launch({ args: [] });
const chromiumVersion = browser.version();
const context = await browser.newContext({ viewport: { width: 1366, height: 768 }, deviceScaleFactor: 1, locale: locale === 'zh-CN' ? 'zh-CN' : 'en-US' });
await context.addInitScript(HIDE_GPU);
if (scenario === 'idle') await context.addInitScript(GL_COUNT);
const page = await context.newPage();
const consoleMsgs = []; page.on('console', (m) => consoleMsgs.push(`${m.type()}: ${m.text()}`)); page.on('pageerror', (e) => consoleMsgs.push('pageerror: ' + e.message));
const q = new URLSearchParams({ scenario, variant, mirror, specimen, locale });
const tGoto = Date.now();
await page.goto(`${base}/index.html?${q}`);
await page.waitForFunction(() => window.__pvcReady === true || (window.__pvcMeta && window.__pvcMeta.error), null, { timeout: 60000 });
const gotoToReady = Date.now() - tGoto;
const pmeta0 = await page.evaluate(() => ({ ...window.__pvcMeta }));
if (pmeta0.error) throw new Error('page boot error: ' + pmeta0.error);
if (pmeta0.navigator_gpu !== 'undefined') throw new Error('navigator.gpu not hidden: ' + pmeta0.navigator_gpu);
const cdp = await context.newCDPSession(page);
const nFrames = () => page.evaluate(() => window.__pvcFrames.length);
async function settle(quietMs = 150) { let last = -1, since = Date.now(); for (;;) { const n = await nFrames(); if (n !== last) { last = n; since = Date.now(); } else if (Date.now() - since >= quietMs) return n; await sleep(20); } }
const R = await page.evaluate(() => { window.__pvcRectsRefresh && window.__pvcRectsRefresh(); return window.__pvcRects; });
let eventsSent = 0;
const idle = {};

// ---------- scenarios (METHOD §2.1) ----------
const S = {
  async 'ptr-move'() {
    const inn = centre(R.connect_button), out = centre(R.bio_text);
    await page.mouse.move(...inn); await settle();
    for (let i = 0; i < 200; i++) { await page.mouse.move(...(i % 2 === 0 ? out : inn)); eventsSent++; await sleep(GAP); }
    await settle();
  },
  async tab() {
    await page.mouse.click(20, 20); await settle();
    for (let i = 0; i < 60; i++) { await page.keyboard.press('Tab'); eventsSent++; await sleep(GAP); }
    await settle();
  },
  async 'type-latin'() {
    await page.mouse.click(...centre(R.name)); await settle();
    for (let round = 0; round < 5; round++) {
      for (const ch of 'Ada Lovelace Byron 1') { await page.keyboard.type(ch); eventsSent++; await sleep(GAP); }
      await settle();
      await page.keyboard.press('Control+A'); await page.keyboard.press('Delete');
      for (let i = 0; i < 30; i++) await page.keyboard.press('Backspace');
      await settle();
    }
  },
  async 'ime-zh'() {
    await page.mouse.click(...centre(R.name)); await settle();
    for (let round = 0; round < 20; round++) {
      for (const pre of ['n', 'ni', 'nih', 'niha', 'nihao']) { await cdp.send('Input.imeSetComposition', { text: pre, selectionStart: pre.length, selectionEnd: pre.length }); eventsSent++; await sleep(GAP); }
      await cdp.send('Input.insertText', { text: '你好' }); eventsSent++; await sleep(GAP);
      await settle();
      await page.keyboard.press('Backspace'); await page.keyboard.press('Backspace'); await settle();
    }
  },
  async 'toggle-show'() {
    await page.mouse.move(...centre(R.toggle)); await settle();
    for (let i = 0; i < 100; i++) { await page.mouse.down(); eventsSent++; await sleep(GAP / 2); await page.mouse.up(); eventsSent++; await sleep(GAP / 2); }
    await settle();
  },
  async checkbox() {
    await page.mouse.move(...centre(R.checkbox)); await settle();
    for (let i = 0; i < 100; i++) { await page.mouse.down(); eventsSent++; await sleep(GAP / 2); await page.mouse.up(); eventsSent++; await sleep(GAP / 2); }
    await settle();
  },
  async idle() {
    // 1 s settle after the first frame, then 5 s with no input at all (METHOD §6). Page clock = performance.now().
    await sleep(1000);
    const t0 = await page.evaluate(() => performance.now()); const g0 = await page.evaluate(() => window.__glCalls.length);
    await sleep(5000);
    const t1 = await page.evaluate(() => performance.now()); const g1 = await page.evaluate(() => window.__glCalls.length);
    idle.window_start_ns = Math.round(t0 * 1e6); idle.window_end_ns = Math.round(t1 * 1e6); idle.gl_calls_in_window = g1 - g0;
    idle.idle_state = 'idle-initial';
    if (specimen === 'F01') { // informational: focused field (not part of the pass)
      await page.mouse.click(...centre(R.name)); await settle(300);
      const h0 = await page.evaluate(() => performance.now()); const gh0 = await page.evaluate(() => window.__glCalls.length);
      await sleep(5000);
      const h1 = await page.evaluate(() => performance.now()); const gh1 = await page.evaluate(() => window.__glCalls.length);
      const fr = await page.evaluate(([a, b]) => window.__pvcFrames.filter((f) => f.t_ns >= a * 1e6 && f.t_ns < b * 1e6).length, [h0, h1]);
      idle.focused_info_only = { frames_in_5s: fr, gl_calls_in_5s: gh1 - gh0 };
    }
  },
  async 'first-frame'() { /* nothing after load */ },
};
const ALL = { L01: ['ptr-move'], F01: ['tab', 'type-latin', 'ime-zh', 'toggle-show', 'checkbox'] };
let scenariosRun = [scenario];
if (scenario === 'rss') { scenariosRun = ALL[specimen]; for (const s of scenariosRun) await S[s](); await sleep(500); }
else await S[scenario]();

// ---------- collect ----------
const frames = await page.evaluate(() => window.__pvcFrames.slice());
const pmeta = await page.evaluate(() => ({ ...window.__pvcMeta }));
const HDR = ['frame_idx', 't_ns', 'scenario', 'event', 'update_ms', 'layout_ms', 'scene_ms', 'a11y_ms', 'mirror_ms', 'render_ms', 'total_ms', 'completion_ms'];
const cell = (v) => (v === null || v === undefined || (typeof v === 'number' && Number.isNaN(v)) ? '' : v);
fs.writeFileSync(path.join(outDir, `trace_rep${rep}.csv`), HDR.join(',') + '\n' + frames.map((f) => HDR.map((h) => cell(f[h])).join(',')).join('\n') + '\n');

// ---------- RSS (METHOD §8): renderer + GPU processes of this Chromium ----------
function procs() {
  const all = {};
  for (const d of fs.readdirSync('/proc')) { if (!/^\d+$/.test(d)) continue; try { const st = fs.readFileSync(`/proc/${d}/stat`, 'utf8'); const ppid = +st.slice(st.lastIndexOf(')') + 2).split(' ')[1]; all[d] = { pid: +d, ppid, cmd: fs.readFileSync(`/proc/${d}/cmdline`, 'utf8').replace(/\0/g, ' ') }; } catch (e) {} }
  return all;
}
function kb(pid, key) { try { const m = fs.readFileSync(`/proc/${pid}/status`, 'utf8').match(new RegExp(`^${key}:\\s+(\\d+)`, 'm')); return m ? +m[1] : 0; } catch (e) { return 0; } }
function pss(pid) { try { const m = fs.readFileSync(`/proc/${pid}/smaps_rollup`, 'utf8').match(/^Pss:\s+(\d+)/m); return m ? +m[1] : 0; } catch (e) { return 0; } }
const P = procs();
// Playwright 1.56 has no browser.process(): the Chromium root is the child of this node process whose cmdline is the browser binary
const roots = Object.values(P).filter((p) => p.ppid === process.pid && /chrom|headless_shell/.test(p.cmd)).map((p) => String(p.pid));
const desc = new Set(roots); let grew = true;
while (grew) { grew = false; for (const p of Object.values(P)) if (!desc.has(String(p.pid)) && desc.has(String(p.ppid))) { desc.add(String(p.pid)); grew = true; } }
const rss = { method: 'web-rss = sum VmRSS(renderer + gpu-process) of this Chromium, read at end of run', procs: [] };
let sumRss = 0, sumHwm = 0, sumPss = 0, browserRss = 0;
for (const d of desc) { const c = P[d].cmd; const type = (c.match(/--type=([\w-]+)/) || [, 'browser'])[1]; const r = { pid: +d, type, vmrss_kb: kb(d, 'VmRSS'), vmhwm_kb: kb(d, 'VmHWM'), pss_kb: pss(d) }; rss.procs.push(r);
  if (type === 'renderer' || type === 'gpu-process') { sumRss += r.vmrss_kb; sumHwm += r.vmhwm_kb; sumPss += r.pss_kb; } if (type === 'browser') browserRss += r.vmrss_kb; }
Object.assign(rss, { web_rss_renderer_gpu_sum_kb: sumRss, web_hwm_renderer_gpu_sum_kb: sumHwm, web_pss_renderer_gpu_sum_kb: sumPss, browser_process_rss_kb: browserRss });
const probeAfter = await idleProbe(1500);

const cpu = process.cpuUsage();
await browser.close(); srv.close();
fs.writeFileSync(path.join(outDir, `runmeta_rep${rep}.json`), JSON.stringify({
  path: 'chromium141-webgl2-swiftshader', specimen, scenario, variant, mirror, mode: null, rep: +rep, cmd,
  date_utc: new Date().toISOString(), loadavg_start: loadavg, idle_probe_before_pct: probe, idle_probe_after_pct: probeAfter, noisy,
  events_sent: eventsSent, scenarios_run: scenariosRun, artifacts_sha256: artifacts, toolchain: { chromium: chromiumVersion, node: process.version },
  page_meta: pmeta, goto_to_first_row_ms: gotoToReady, idle: Object.keys(idle).length ? idle : undefined, idle_state: idle.idle_state, rss,
  harness_cpu_s: +((cpu.user + cpu.system) / 1e6).toFixed(2), console: consoleMsgs.slice(0, 20), frames_recorded: frames.length,
}, null, 1));
console.log(`rep ${rep} ${specimen}/${scenario}: frames=${frames.length} events_sent=${eventsSent} noisy=${noisy}`);
