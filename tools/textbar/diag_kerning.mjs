// Diagnostic (informational, not part of the bar): text widths in Chromium/Firefox with default vs font-kerning:none vs text-rendering variants.
import http from 'node:http'; import fs from 'node:fs'; import { createRequire } from 'node:module';
const require = createRequire('/home/claude/.npm-global/lib/node_modules/'); const { chromium, firefox } = require('playwright');
const F='/workspace/dev_work/dev-render/pvc-runtime/fonts/';
const srv=http.createServer((q,r)=>{const f=q.url.slice(1);if(f==='favicon.ico'){r.statusCode=404;r.end();return}if(f==='p'){r.setHeader('content-type','text/html');r.end(HTML);return}r.end(fs.readFileSync(F+f))});
await new Promise(r=>srv.listen(0,'127.0.0.1',r)); const base=`http://127.0.0.1:${srv.address().port}`;
const T='WAVE AVATAR Typography: fi fl ffi ffl office waffle affinity.';
var HTML=`<style>@font-face{font-family:L;src:url(${base}/DejaVuSans.ttf)}span{font:11px L;white-space:nowrap}</style>
<span id=a>${T}</span><br><span id=b style="font-kerning:none">${T}</span><br><span id=c style="text-rendering:geometricPrecision">${T}</span><br><span id=d style="font-variant-ligatures:none">${T}</span>`;
for (const [n,bt,o] of [['chromium',chromium,{args:['--disable-lcd-text']}],['firefox',firefox,{executablePath:'/workspace/dev_work/dev-measure/pw/firefox-1495/firefox/firefox'}]]) {
  const b=await bt.launch(o); for (const dsf of [1,2]) { const p=await (await b.newContext({deviceScaleFactor:dsf})).newPage(); await p.goto(base+'/p'); await p.evaluate(()=>document.fonts.ready);
  console.log(n,'dsf',dsf,await p.evaluate(()=>['a','b','c','d'].map(i=>i+':'+document.getElementById(i).getBoundingClientRect().width.toFixed(2)).join(' '))); } await b.close(); }
srv.close();
