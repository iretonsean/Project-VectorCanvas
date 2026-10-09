// Renders corpus.json with Chromium or Firefox (Playwright), same font files via @font-face,
// writes PNG per (script,size,scale) and line-break JSON. See METHOD_TEXTBAR.md.
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
const require = createRequire('/home/claude/.npm-global/lib/node_modules/');
const { chromium, firefox } = require('playwright');

const engine = process.argv[2];            // chromium | firefox
const outDir = process.argv[3];            // e.g. .../text-bar/work/chromium
const corpusPath = process.argv[4];
const FONTS = '/workspace/dev_work/dev-render/pvc-runtime/fonts/';
const SIZES = [11, 13, 16, 24];
const SCALES = [1, 2];
const corpus = JSON.parse(fs.readFileSync(corpusPath, 'utf8'));
const W = corpus.wrap_width_css_px;
fs.mkdirSync(outDir, { recursive: true });

const fontFiles = {
  'DejaVuSans.ttf': 'font/ttf', 'DejaVuSans-Bold.ttf': 'font/ttf',
  'NotoSansCJKsc-Regular.otf': 'font/otf',
};
let currentHtml = '';
const server = http.createServer((req, res) => {
  const u = decodeURIComponent(req.url.split('?')[0]);
  if (u === '/page.html') { res.setHeader('content-type', 'text/html; charset=utf-8'); res.end(currentHtml); return; }
  const f = path.basename(u);
  if (fontFiles[f]) { res.setHeader('content-type', fontFiles[f]); res.end(fs.readFileSync(FONTS + f)); return; }
  res.statusCode = 404; res.end('no');
});
await new Promise(r => server.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${server.address().port}`;

const esc = s => s.replace(/&/g, '&amp;').replace(/</g, '&lt;');
function buildHtml(script, size) {
  const lh = Math.round(1.35 * size);
  const items = corpus.items.filter(i => i.script === script);
  let top = 0; let body = '';
  const layout = [];
  for (const it of items) {
    const slot = it.slot_lines * lh + 8;
    const wrap = it.kind === 'paragraph';
    body += `<div class="it" id="${it.id}" lang="${script === 'zh-CN' ? 'zh-CN' : 'en'}" style="top:${top + 4}px;font-weight:${it.weight};${wrap ? `width:${W}px;white-space:normal;` : 'white-space:nowrap;'}">${esc(it.text)}</div>\n`;
    layout.push({ id: it.id, top, slot });
    top += slot;
  }
  const html = `<!doctype html><html><head><meta charset="utf-8"><style>
@font-face{font-family:PVCLatin;src:url(${base}/DejaVuSans.ttf);font-weight:400}
@font-face{font-family:PVCLatin;src:url(${base}/DejaVuSans-Bold.ttf);font-weight:700}
@font-face{font-family:PVCCJK;src:url(${base}/NotoSansCJKsc-Regular.otf);font-weight:400}
html,body{margin:0;padding:0;background:#fff}
body{width:800px;height:${top}px;position:relative;overflow:hidden}
.it{position:absolute;left:8px;color:#000;font-family:PVCLatin,PVCCJK;font-size:${size}px;line-height:${lh}px;text-size-adjust:none}
</style></head><body>${body}</body></html>`;
  return { html, height: top, lh };
}

const browserType = engine === 'chromium' ? chromium : firefox;
const launchOpts = engine === 'firefox'
  ? { executablePath: '/workspace/dev_work/dev-measure/pw/firefox-1495/firefox/firefox' }
  : {};
if (process.env.PVC_BROWSER_ARGS) launchOpts.args = process.env.PVC_BROWSER_ARGS.split(' ');
const browser = await browserType.launch(launchOpts);
const version = browser.version();
const results = { engine, version, cells: {} };

for (const script of ['latin', 'zh-CN']) {
  for (const size of SIZES) {
    const { html, height, lh } = buildHtml(script, size);
    currentHtml = html;
    for (const scale of SCALES) {
      const ctx = await browser.newContext({ viewport: { width: 800, height }, deviceScaleFactor: scale });
      const page = await ctx.newPage();
      await page.goto(base + '/page.html');
      await page.evaluate(async () => { await document.fonts.ready; });
      const fonts = await page.evaluate(() => [...document.fonts].map(f => `${f.family}:${f.weight}:${f.status}`));
      const breaks = await page.evaluate(() => {
        const out = {};
        for (const el of document.querySelectorAll('.it')) {
          const tn = el.firstChild; const t = tn.textContent;
          const starts = []; let lastTop = null; const lineTops = [];
          for (let i = 0; i < t.length; i++) {
            if (/\s/.test(t[i])) continue;
            const r = document.createRange(); r.setStart(tn, i); r.setEnd(tn, i + 1);
            const rects = r.getClientRects(); if (!rects.length) continue;
            const top = Math.round(rects[0].top * 100) / 100;
            if (lastTop === null || Math.abs(top - lastTop) > 1) { starts.push(i); lastTop = top; lineTops.push(top); }
          }
          out[el.id] = { starts, lineTops };
        }
        return out;
      });
      const png = `${outDir}/${script}_s${size}_x${scale}.png`;
      await page.screenshot({ path: png, clip: { x: 0, y: 0, width: 800, height }, animations: 'disabled' });
      results.cells[`${script}_s${size}_x${scale}`] = { png: path.basename(png), height, lh, fonts, breaks };
      await ctx.close();
    }
  }
}
fs.writeFileSync(`${outDir}/breaks_${path.basename(outDir)}.json`, JSON.stringify(results, null, 1));
await browser.close(); server.close();
console.log(engine, version, 'ok');
