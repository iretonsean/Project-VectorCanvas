// A7 web path: Chromium 141 headless, navigator.gpu hidden, WebGL2 (ANGLE->SwiftShader). Writes
// <out>/<state>.web-webgl2.png and fingerprints.web.json. usage: node run_golden_web.mjs <out_dir>
import { chromium } from '/opt/node22/lib/node_modules/playwright/index.mjs';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
const out = process.argv[2];
fs.mkdirSync(out, { recursive: true });
const root = path.join(path.dirname(new URL(import.meta.url).pathname), 'www');
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.pvca': 'application/octet-stream' };
const srv = http.createServer((q, r) => {
  const p = path.join(root, decodeURIComponent(q.url.split('?')[0]).replace(/^\/$/, '/index.html'));
  if (!p.startsWith(root) || !fs.existsSync(p)) { r.writeHead(404); r.end(); return; }
  r.writeHead(200, { 'content-type': types[path.extname(p)] || 'application/octet-stream' });
  r.end(fs.readFileSync(p));
}).listen(0, '127.0.0.1');
await new Promise((r) => srv.on('listening', r));
const port = srv.address().port;
const browser = await chromium.launch({});
const ctx = await browser.newContext({ viewport: { width: 1366, height: 768 }, deviceScaleFactor: 1 });
await ctx.addInitScript(() => { try { Object.defineProperty(Navigator.prototype, 'gpu', { get: () => undefined, configurable: true }); } catch (e) {} });
const page = await ctx.newPage();
const msgs = [];
page.on('console', (m) => msgs.push(`${m.type()}: ${m.text()}`));
page.on('pageerror', (e) => msgs.push('pageerror: ' + e.message));
await page.goto(`http://127.0.0.1:${port}/index.html`);
await page.waitForFunction(() => window.__ready || window.__error, null, { timeout: 120000 });
const err = await page.evaluate(() => window.__error);
if (err) { console.error('boot error', err, msgs); process.exit(1); }
const info = await page.evaluate(() => ({ gpu: typeof navigator.gpu, ua: navigator.userAgent }));
const results = { navigator_gpu: info.gpu, user_agent: info.ua, browser_version: browser.version(), states: {} };
for (const s of ['L01_idle', 'F01_empty', 'F01_error', 'F01_zh_preedit']) {
  const m = await page.evaluate((s) => window.__golden(s), s);
  const b64 = m.png.replace(/^data:image\/png;base64,/, '');
  fs.writeFileSync(path.join(out, `${s}.web-webgl2.png`), Buffer.from(b64, 'base64'));
  delete m.png;
  results.states[s] = m;
}
results.console = msgs;
fs.writeFileSync(path.join(out, 'fingerprints.web.json'), JSON.stringify(results, null, 1));
console.log(JSON.stringify(results, null, 1));
await browser.close();
srv.close();
