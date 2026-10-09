// examples/make-fails.mjs — writes examples/<stack>/<ID>.fail-<name>.{html,tsx} from the passing examples + mutations.mjs
import fs from 'node:fs';
import { MUTATIONS } from './mutations.mjs';
const b2 = s => s.replace(/\bclass=/g, 'className=').replace(/\bfor=/g, 'htmlFor=');
let n = 0;
for (const m of MUTATIONS) {
  for (const stack of ['B1', 'B2']) {
    const ext = stack === 'B1' ? 'html' : 'tsx';
    const src = `${stack === 'B1' ? 'B1' : 'B2'}/${m.id}.pass.${ext}`;
    let text = fs.readFileSync(new URL(src, import.meta.url), 'utf8');
    const pairs = stack === 'B1' ? m.pairs : (m.B2 ?? m.pairs.map(([f, r]) => [b2(f), b2(r)]));
    if (stack === 'B1' && m.name === 'type-error') continue; // B2-only (TypeScript)
    for (const [f, r] of pairs) {
      if (!text.includes(f)) throw new Error(`${m.id}/${m.name} ${stack}: find string not present: ${f.slice(0, 60)}`);
      text = text.split(f).join(r);
    }
    fs.writeFileSync(new URL(`${stack}/${m.id}.fail-${m.name}.${ext}`, import.meta.url), text); n++;
  }
}
console.log('wrote', n, 'failing examples');
