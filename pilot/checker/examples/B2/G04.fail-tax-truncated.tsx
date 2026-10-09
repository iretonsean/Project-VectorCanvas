import { useState, type FormEvent } from 'react';

type Item = { n: string; c: number; q: number };
const money = (c: number) => '$' + Math.floor(c / 100) + '.' + String(c % 100).padStart(2, '0');

export default function App() {
  const [items, setItems] = useState<Item[]>([{ n: 'Notebook', c: 499, q: 1 }, { n: 'Pen', c: 125, q: 1 }, { n: 'Backpack', c: 1990, q: 1 }]);
  const [promo, setPromo] = useState(false);
  const [code, setCode] = useState('');
  const [err, setErr] = useState('');
  const sub = items.reduce((a, i) => a + i.c * i.q, 0);
  const disc = promo ? Math.floor((sub * 10 + 50) / 100) : 0;
  const base = sub - disc;
  const tax = Math.floor(base * 825 / 10000);
  const upd = (n: string, d: number) => setItems(items.map((i) => (i.n === n ? { ...i, q: Math.max(1, i.q + d) } : i)));
  const apply = (e: FormEvent) => { e.preventDefault(); if (code.trim() === 'SAVE10') { setPromo(true); setErr(''); } else setErr('Invalid promo code'); };
  return (
    <main className="[overflow-wrap:anywhere] max-w-lg mx-auto bg-white text-slate-900 p-4">
      <h1 className="text-2xl font-bold mb-4">Cart</h1>
      <ul className="flex flex-col gap-2 mb-4">
        {items.map((it) => (
          <li key={it.n} className="flex items-center gap-2 flex-wrap">
            <span className="flex-1 min-w-[6rem]">{it.n}</span>
            <span>{money(it.c)}</span>
            <button aria-label="Decrease quantity" className="border border-slate-700 rounded px-2" onClick={() => upd(it.n, -1)}>−</button>
            <span aria-label="Quantity">{it.q}</span>
            <button aria-label="Increase quantity" className="border border-slate-700 rounded px-2" onClick={() => upd(it.n, 1)}>+</button>
            <button aria-label="Remove" className="border border-slate-700 rounded px-2" onClick={() => setItems(items.filter((x) => x.n !== it.n))}>Remove</button>
          </li>
        ))}
      </ul>
      <form onSubmit={apply} className="flex gap-2 mb-4">
        <label htmlFor="code" className="sr-only">Promo code</label>
        <input id="code" value={code} onChange={(e) => setCode(e.target.value)} className="flex-1 min-w-0 border border-slate-500 rounded px-2 py-1" placeholder="Promo code" />
        <button className="border border-slate-700 rounded px-3 py-1">Apply</button>
      </form>
      <p role="alert" className="text-red-800 mb-2">{err}</p>
      <dl className="grid grid-cols-2 gap-1">
        <dt>Subtotal</dt><dd className="text-right">{money(sub)}</dd>
        <dt>Discount</dt><dd className="text-right">{money(disc)}</dd>
        <dt>Tax</dt><dd className="text-right">{money(tax)}</dd>
        <dt className="font-bold">Total</dt><dd className="text-right font-bold">{money(base + tax)}</dd>
      </dl>
    </main>
  );
}
