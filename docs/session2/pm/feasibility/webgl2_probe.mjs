import { chromium } from '/opt/node22/lib/node_modules/playwright/index.mjs';
for (const args of [[], ['--use-angle=swiftshader','--enable-unsafe-swiftshader'], ['--use-gl=angle','--use-angle=swiftshader-webgl']]) {
  const b = await chromium.launch({ args });
  const p = await b.newPage();
  await p.addInitScript(() => { try { Object.defineProperty(Navigator.prototype, 'gpu', { get: () => undefined, configurable: true }); } catch(e){} });
  await p.setContent('<canvas id=c width=320 height=240></canvas>');
  const r = await p.evaluate(() => {
    const gl = document.getElementById('c').getContext('webgl2');
    if (!gl) return { webgl2: false, gpu: typeof navigator.gpu };
    const ext = gl.getExtension('WEBGL_debug_renderer_info');
    return { webgl2: true, gpu: typeof navigator.gpu, version: gl.getParameter(gl.VERSION),
      renderer: ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER),
      maxTex: gl.getParameter(gl.MAX_TEXTURE_SIZE) };
  });
  console.log(JSON.stringify({ args, version: b.version(), ...r }));
  await b.close();
}
