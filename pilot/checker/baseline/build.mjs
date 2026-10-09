// baseline/build.mjs — offline builds for the two baselines (G2 §3.3).
//   B1: one HTML file (+Tailwind, vanilla JS). Tailwind is compiled LOCALLY with the Tailwind v3 CLI from the classes
//       in the file (replaces the CDN <script>, which cannot load: no network at check time).
//   B2: React + TypeScript + Tailwind (+ MUI available). tsc type-check, esbuild bundle, Tailwind CLI.
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const BIN = n => path.join(HERE, 'node_modules', '.bin', n);
export const WORK = path.join(HERE, 'work');

function sh(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, ...opts });
  return { code: r.status, out: (r.stdout || '') + (r.stderr || '') };
}
function workdir(tag, content) {
  const h = crypto.createHash('sha1').update(content).digest('hex').slice(0, 10);
  const d = path.join(WORK, `${tag}-${h}-${process.pid}`);
  fs.rmSync(d, { recursive: true, force: true });
  fs.mkdirSync(d, { recursive: true });
  return d;
}

function runTailwind(dir, contentGlobs, extraCss = '', cfgObj = {}) {
  const cfg = { ...cfgObj, content: contentGlobs.map(g => path.resolve(g)) };
  fs.writeFileSync(path.join(dir, 'tailwind.config.cjs'), 'module.exports = ' + JSON.stringify(cfg) + ';');
  fs.writeFileSync(path.join(dir, 'in.css'), '@tailwind base;\n@tailwind components;\n@tailwind utilities;\n' + extraCss);
  const r = sh(BIN('tailwindcss'), ['-c', path.join(dir, 'tailwind.config.cjs'), '-i', path.join(dir, 'in.css'), '-o', path.join(dir, 'out.css')], { cwd: dir });
  const ok = r.code === 0 && fs.existsSync(path.join(dir, 'out.css'));
  return { ok, log: r.out, css: ok ? fs.readFileSync(path.join(dir, 'out.css'), 'utf8') : '' };
}

export function buildB1(file) {
  const src = fs.readFileSync(file, 'utf8');
  const dir = workdir('b1', src);
  const errors = [];
  let html = src;
  // 1. Tailwind CDN config scripts -> theme config (what the CDN's JIT would have read)
  let cfg = {};
  const scriptRe = /<script\b([^>]*)>([\s\S]*?)<\/script>/gi;
  html = html.replace(scriptRe, (m, attrs, body) => {
    if (/src\s*=\s*["'][^"']*tailwind[^"']*["']/i.test(attrs)) return '<!-- tailwind cdn script replaced by local build -->';
    if (/tailwind\.config\s*=/.test(body)) {
      try {
        const sb = { tailwind: {}, window: {} };
        vm.createContext(sb); vm.runInContext(body, sb, { timeout: 500 });
        const c = sb.tailwind.config ?? sb.window.tailwind?.config;
        if (c && typeof c === 'object') cfg = JSON.parse(JSON.stringify(c));
      } catch (e) { errors.push('tailwind.config script could not be evaluated: ' + e.message); }
      return '<!-- tailwind.config script consumed by local build -->';
    }
    return m;
  });
  // 2. <style type="text/tailwindcss"> -> extra css for the CLI
  let extra = '';
  html = html.replace(/<style\b[^>]*type\s*=\s*["']text\/tailwindcss["'][^>]*>([\s\S]*?)<\/style>/gi, (m, body) => { extra += body + '\n'; return ''; });
  const f = path.join(dir, 'index.src.html');
  fs.writeFileSync(f, src);
  const tw = runTailwind(dir, [f], extra, cfg);
  if (!tw.ok) errors.push('tailwind build failed: ' + tw.log.split('\n').filter(Boolean).slice(0, 5).join(' | '));
  const css = `<style id="__tw_local">${tw.css}</style>`;
  const shim = '<script>window.tailwind=window.tailwind||{};</script>';
  if (/<head[^>]*>/i.test(html)) html = html.replace(/<head[^>]*>/i, m => m + shim + css);
  else if (/<html[^>]*>/i.test(html)) html = html.replace(/<html[^>]*>/i, m => m + '<head>' + shim + css + '</head>');
  else html = '<!doctype html><html><head><meta charset="utf-8">' + shim + css + '</head><body>' + html + '</body></html>';
  fs.writeFileSync(path.join(dir, 'index.html'), html);
  return { ok: tw.ok, dir, root: dir, errors, tsc: [], kind: 'B1' };
}

export function buildB2(fileOrDir) {
  const st = fs.statSync(fileOrDir);
  const srcFiles = [];
  if (st.isDirectory()) {
    for (const f of fs.readdirSync(fileOrDir)) if (/\.(tsx?|css)$/.test(f)) srcFiles.push(path.join(fileOrDir, f));
  } else srcFiles.push(fileOrDir);
  const content = srcFiles.map(f => fs.readFileSync(f, 'utf8')).join('\n');
  const dir = workdir('b2', content);
  const src = path.join(dir, 'src'); fs.mkdirSync(src);
  let hasApp = false;
  for (const f of srcFiles) {
    let base = path.basename(f);
    if (st.isFile()) base = 'App.tsx';
    if (base === 'App.tsx') hasApp = true;
    fs.copyFileSync(f, path.join(src, base));
  }
  const errors = [];
  if (!hasApp) return { ok: false, dir, root: dir, errors: ['no App.tsx found (B2 output must provide App.tsx with a default-exported App component)'], tsc: [], kind: 'B2' };
  fs.writeFileSync(path.join(src, 'main.tsx'),
    `import { createRoot } from 'react-dom/client';\nimport App from './App';\nimport './tw.css';\ncreateRoot(document.getElementById('root')!).render(<App />);\n`);
  fs.writeFileSync(path.join(dir, 'tsconfig.json'), JSON.stringify({
    compilerOptions: { target: 'ES2022', lib: ['ES2022', 'DOM', 'DOM.Iterable'], module: 'ESNext', moduleResolution: 'bundler', jsx: 'react-jsx', strict: true, noEmit: true, skipLibCheck: true, esModuleInterop: true, types: [], allowImportingTsExtensions: true },
    include: ['src'],
  }));
  // Tailwind first (main.tsx imports tw.css). Content = the agent's sources.
  const tw = runTailwind(dir, [path.join(src, '*.tsx'), path.join(src, '*.ts')], '', {});
  if (!tw.ok) errors.push('tailwind build failed: ' + tw.log.split('\n').filter(Boolean).slice(0, 5).join(' | '));
  fs.writeFileSync(path.join(src, 'tw.css'), tw.css);
  // tsc
  const tsc = [];
  const r = sh(BIN('tsc'), ['-p', path.join(dir, 'tsconfig.json'), '--pretty', 'false'], { cwd: dir });
  for (const line of r.out.split('\n')) {
    const m = /^src\/(.+?)\((\d+),(\d+)\): error (TS\d+): (.*)$/.exec(line);
    if (m) tsc.push({ file: m[1], line: +m[2], col: +m[3], code: m[4], msg: m[5].slice(0, 220) });
    else if (/error TS\d+/.test(line)) tsc.push({ file: '-', line: 0, col: 0, code: (/(TS\d+)/.exec(line) || [])[1], msg: line.slice(0, 220) });
  }
  // esbuild
  fs.mkdirSync(path.join(dir, 'dist'));
  const eb = sh(BIN('esbuild'), [path.join(src, 'main.tsx'), '--bundle', '--format=iife', '--jsx=automatic', '--define:process.env.NODE_ENV="production"', '--loader:.css=css', `--outdir=${path.join(dir, 'dist')}`, '--log-level=error', '--minify'], { cwd: dir });
  let bundled = eb.code === 0 && fs.existsSync(path.join(dir, 'dist', 'main.js'));
  if (!bundled) errors.push('bundle failed: ' + eb.out.split('\n').filter(Boolean).slice(0, 6).join(' | '));
  const cssLink = fs.existsSync(path.join(dir, 'dist', 'main.css')) ? '<link rel="stylesheet" href="main.css">' : '';
  fs.writeFileSync(path.join(dir, 'dist', 'index.html'),
    `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>app</title>${cssLink}</head><body><div id="root"></div><script src="main.js"></script></body></html>`);
  return { ok: tw.ok && bundled && tsc.length === 0, dir: path.join(dir, 'dist'), root: dir, errors, tsc, kind: 'B2', bundled };
}

export function cleanup(res) { try { fs.rmSync(res.root, { recursive: true, force: true }); } catch (e) { /* */ } }
