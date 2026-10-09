import { useRef, useState, type KeyboardEvent } from 'react';

const TABS = ['Profile', 'Notes'] as const;

export default function App() {
  const [sel, setSel] = useState(0);
  const [text, setText] = useState(['', '']);
  const [dirty, setDirty] = useState([false, false]);
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const select = (i: number) => {
    if (i === sel) return;
    if (dirty[sel]) {
      if (!window.confirm('Discard changes?')) return;
      setText(text.map((t, k) => (k === sel ? '' : t)));
      setDirty(dirty.map((d, k) => (k === sel ? false : d)));
    }
    setSel(i);
  };
  const onKey = (e: KeyboardEvent, i: number) => {
    const d = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
    if (d) { e.preventDefault(); refs.current[(i + d + TABS.length) % TABS.length]?.focus(); }
  };
  return (
    <main className="[overflow-wrap:anywhere] max-w-lg mx-auto bg-white text-slate-900 p-4">
      <div role="tablist" aria-label="Sections" className="flex gap-2 border-b border-slate-400">
        {TABS.map((t, i) => (
          <button key={t} ref={(el) => { refs.current[i] = el; }} role="tab" id={'t' + i} aria-selected={sel === i} aria-controls={'p' + i} tabIndex={sel === i ? 0 : -1}
            onClick={() => select(i)} onKeyDown={(e) => onKey(e, i)}
            className={'px-3 py-2 border-b-4 ' + (sel === i ? 'border-blue-700 font-semibold' : 'border-transparent')}>{t}</button>
        ))}
      </div>
      {TABS.map((t, i) => (
        <div key={t} role="tabpanel" id={'p' + i} aria-labelledby={'t' + i} hidden={sel !== i} className="py-4">
          <label htmlFor={'a' + i} className="block mb-1">{t} text</label>
          <textarea id={'a' + i} rows={4} value={text[i]} onChange={(e) => { setText(text.map((x, k) => (k === i ? e.target.value : x))); setDirty(dirty.map((d, k) => (k === i ? true : d))); }} className="w-full border border-slate-500 rounded p-2" />
        </div>
      ))}
    </main>
  );
}
