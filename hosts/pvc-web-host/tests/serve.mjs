// Static file server for www/ with a request log. Same-origin static assets only (hard-wall evidence).
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.otf': 'font/otf', '.ttf': 'font/ttf', '.signed': 'application/octet-stream', '.bin': 'application/octet-stream' };
export function serve(root, port = 0, logFile = null) {
  const log = [];
  const srv = http.createServer((req, res) => {
    const u = new URL(req.url, 'http://x');
    let p = path.normalize(path.join(root, decodeURIComponent(u.pathname)));
    if (!p.startsWith(root)) { res.writeHead(403); return res.end(); }
    if (fs.existsSync(p) && fs.statSync(p).isDirectory()) p = path.join(p, 'index.html');
    const ok = fs.existsSync(p);
    log.push({ method: req.method, url: u.pathname + u.search, status: ok ? 200 : 404, ua: undefined });
    if (!ok) { res.writeHead(404); return res.end('not found'); }
    res.writeHead(200, { 'content-type': MIME[path.extname(p)] || 'application/octet-stream', 'cache-control': 'no-store' });
    fs.createReadStream(p).pipe(res);
  });
  return new Promise((resolve) => srv.listen(port, '127.0.0.1', () => resolve({
    port: srv.address().port, log,
    close: () => new Promise((r) => { if (logFile) fs.writeFileSync(logFile, JSON.stringify(log, null, 1)); srv.close(r); }),
  })));
}
if (process.argv[1]?.endsWith('serve.mjs')) {
  const s = await serve(path.resolve(process.argv[2] || 'www'), Number(process.argv[3] || 8765));
  console.log('serving on', s.port);
}
