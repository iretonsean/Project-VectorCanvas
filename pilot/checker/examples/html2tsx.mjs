// examples/html2tsx.mjs — mechanical transcription of a (static) B1 example body into a React/TSX component.
// Used only for the layout specimens whose B1 example has no script; the output is committed, not regenerated at check time.
import fs from 'node:fs';
const [, , src, dst, title] = process.argv;
const html = fs.readFileSync(src, 'utf8');
const m = /<body class="([^"]*)">([\s\S]*?)<\/body>/.exec(html);
if (!m) throw new Error('no body');
let [, cls, inner] = m;
if (/<script/.test(inner)) throw new Error('script in body: write this one by hand');
inner = inner
  .replace(/\bclass=/g, 'className=').replace(/\bfor=/g, 'htmlFor=')
  .replace(/ onsubmit="return false"/g, ' onSubmit={(e) => e.preventDefault()}')
  .replace(/<(input|img|br|hr)\b([^>]*?)(?<!\/)>/g, '<$1$2 />')
  .replace(/\bstroke-width=/g, 'strokeWidth=').replace(/\bfill-rule=/g, 'fillRule=')
  .replace(/\n\s*\n/g, '\n');
const out = `// ${title}: B2 example (React + TypeScript + Tailwind). Mechanical transcription of examples/B1/${src.split('/').pop()}.
export default function App() {
  return (
    <div className="${cls}">${inner}</div>
  );
}
`;
fs.writeFileSync(dst, out);
