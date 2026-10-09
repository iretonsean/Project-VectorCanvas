// Shared Playwright plumbing for pvc-web scenarios. dev-measure may import this module.
import { chromium } from '/opt/node22/lib/node_modules/playwright/index.mjs';
import path from 'node:path';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import { serve } from './serve.mjs';

export const HERE = path.dirname(fileURLToPath(import.meta.url));
export const ROOT = path.resolve(HERE, '..');
export const HOST_LABEL = 'host=container-xeon4 path=chromium141-webgl2-swiftshader';
export const SCOPE_LABELS = [
  'general constraint solve excluded (D-4): layout is the direct sugar solver',
  'NOT a Tier 1 device result; no phone or Chromebook was measured',
  'WebGL2 here is ANGLE->SwiftShader (CPU emulation), not a GPU',
];
export const IME_LABEL = 'browser IME composition pipeline driven via CDP; no OS IME engine involved';

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Hides navigator.gpu exactly as in feasibility/webgl2_probe.mjs. */
export const HIDE_GPU = () => {
  try { Object.defineProperty(Navigator.prototype, 'gpu', { get: () => undefined, configurable: true }); } catch (e) {}
};

/**
 * Launch Chromium + page. opts: {dsf, variant:'raf'|'sync', mirror:'on'|'off', specimen, locale,
 * scenario, debug, viewport, outDir, server (existing), args}
 * Records every request the page makes (hard-wall evidence).
 */
export async function open(opts = {}) {
  const o = { dsf: 1, variant: 'raf', mirror: 'on', specimen: 'F01', locale: 'en', scenario: 'manual', viewport: { width: 1366, height: 768 }, ...opts };
  const server = o.server || (await serve(path.join(ROOT, 'www')));
  const browser = await chromium.launch({ args: o.args || [] });
  const context = await browser.newContext({ viewport: o.viewport, deviceScaleFactor: o.dsf, locale: o.locale === 'zh-CN' ? 'zh-CN' : 'en-US' });
  await context.addInitScript(HIDE_GPU);
  const requests = [];
  context.on('request', (r) => requests.push({ method: r.method(), url: r.url(), resourceType: r.resourceType() }));
  const page = await context.newPage();
  const consoleMsgs = [];
  page.on('console', (m) => consoleMsgs.push(`${m.type()}: ${m.text()}`));
  page.on('pageerror', (e) => consoleMsgs.push(`pageerror: ${e.message}`));
  const q = new URLSearchParams({ scenario: o.scenario, variant: o.variant, mirror: o.mirror, specimen: o.specimen, locale: o.locale });
  if (o.debug) q.set('debug', '1');
  if (o.artifact) q.set('artifact', o.artifact);
  const base = `http://127.0.0.1:${server.port}`;
  const t0 = Date.now();
  await page.goto(`${base}/index.html?${q}`);
  await page.waitForFunction(() => window.__pvcReady === true || (window.__pvcMeta && window.__pvcMeta.error), null, { timeout: 60000 });
  const goto_to_ready_ms = Date.now() - t0;
  
  if (meta.error && !o.allowError) throw new Error('page boot error: ' + meta.error);
  const cdp = await context.newCDPSession(page);
  return {
    loadavg: fs.readFileSync('/proc/loadavg', 'utf8').trim(), o, server, browser, context, page, cdp, requests, consoleMsgs, base, goto_to_ready_ms,
    async close() { await browser.close(); if (!opts.server) await server.close(); },
  };
}

export const rects = (page) => page.evaluate(() => { window.__pvcRectsRefresh && window.__pvcRectsRefresh(); return window.__pvcRects; });
export const frames = (page) => page.evaluate(() => window.__pvcFrames.slice());
export const meta = (page) => page.evaluate(() => ({ ...window.__pvcMeta }));
// runtime debug_state is `key=Variant(value)` lines; parse into a plain object
export const state = async (page) => {
  const j = JSON.parse(await page.evaluate(() => window.__pvc.debug_state()));
  const o = { engine: j.engine, raw: j.raw, focused_input: j.focused_input };
  for (const line of (j.raw || '').split('\n')) {
    const m = line.match(/^(derived )?([\w.]+)=(.*)$/);
    if (!m) continue;
    let v = m[3];
    const t = v.match(/^Text\((".*")\)$/s); const b = v.match(/^Bool\((true|false)\)$/);
    if (t) v = JSON.parse(t[1]); else if (b) v = b[1] === 'true';
    o[(m[1] ? 'derived.' : '') + m[2]] = v;
  }
  return o;
};
export const centre = (r) => [r[0] + r[2] / 2, r[1] + r[3] / 2];

/** Wait until the page has presented no new frame for `quietMs`. */
export async function settle(page, quietMs = 150) {
  let last = -1, since = Date.now();
  for (;;) {
    const n = await page.evaluate(() => window.__pvcFrames.length);
    if (n !== last) { last = n; since = Date.now(); }
    else if (Date.now() - since >= quietMs) return n;
    await sleep(20);
  }
}

/** Full AX tree via CDP (what a screen reader would be given by Chromium). */
export async function axTree(cdp) {
  await cdp.send('Accessibility.enable');
  const { nodes } = await cdp.send('Accessibility.getFullAXTree');
  return nodes;
}

/** Compact, comparable projection of the AX tree: ignored nodes dropped. */
export function axCompact(nodes) {
  const byId = new Map(nodes.map((n) => [n.nodeId, n]));
  const out = [];
  const walk = (n, depth) => {
    if (!n.ignored) {
      const props = {};
      for (const p of n.properties || []) props[p.name] = p.value && p.value.value;
      out.push({ depth, role: n.role && n.role.value, name: n.name && n.name.value, value: n.value && n.value.value, description: n.description && n.description.value, props });
    }
    for (const c of n.childIds || []) walk(byId.get(c), n.ignored ? depth : depth + 1);
  };
  if (nodes.length) walk(nodes[0], 0);
  return out;
}
export const axFind = (compact, role, name) => compact.filter((n) => n.role === role && (name === undefined || n.name === name || (n.name || '').includes(name)));

export function writeJson(file, obj) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, JSON.stringify(obj, null, 1));
}

/** Hard-wall check on the recorded requests: only same-origin GETs of static assets. */
export function requestAudit(requests, base) {
  const origin = new URL(base).origin;
  const bad = requests.filter((r) => !r.url.startsWith(origin + '/') && !r.url.startsWith('data:') && !r.url.startsWith('blob:'));
  const nonGet = requests.filter((r) => r.method !== 'GET');
  const xhrFetch = requests.filter((r) => ['xhr', 'fetch'].includes(r.resourceType)).map((r) => r.url.replace(origin, ''));
  const allowedFetch = /^\/(pkg\/|artifacts\/|fonts\/)/;
  const badFetch = xhrFetch.filter((u) => !allowedFetch.test(u));
  return { total: requests.length, origin, offOrigin: bad, nonGet, fetchXhrUrls: xhrFetch, fetchOutsideStaticAssets: badFetch, pass: bad.length === 0 && nonGet.length === 0 && badFetch.length === 0 };
}
