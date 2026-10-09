import { useState } from 'react';

export default function App() {
  const [open, setOpen] = useState(false);
  return (
    <div className="bg-white text-slate-900 min-h-screen flex flex-col">
      <header className="bg-slate-800 text-white p-4 flex items-center gap-3">
        <button aria-expanded={open} aria-controls="site-nav" onClick={() => setOpen(!open)} className="lg:hidden border border-white rounded px-3 py-1">Menu</button>
        <span className="text-xl font-bold">Acme</span>
      </header>
      <div className="flex-1 flex flex-col lg:flex-row">
        <nav id="site-nav" aria-label="Main" className={(open ? 'block' : 'hidden') + ' lg:block lg:w-48 bg-slate-100 p-4'}>
          <ul className="flex flex-col gap-2">
            <li><a className="text-blue-800 underline" href="#a">Home</a></li>
            <li><a className="text-blue-800 underline" href="#b">Docs</a></li>
            <li><a className="text-blue-800 underline" href="#c">Contact</a></li>
          </ul>
        </nav>
        <main className="flex-1 p-4">
          <h1 className="text-2xl font-bold mb-2">Welcome</h1>
          <p>This is a short page. It has a heading and a couple of sentences.</p>
        </main>
        <div aria-label="Related" className="hidden lg:block lg:w-56 bg-slate-100 p-4">
          <h2 className="font-semibold mb-2">Related</h2>
          <ul className="flex flex-col gap-2"><li><a className="text-blue-800 underline" href="#d">Guide</a></li><li><a className="text-blue-800 underline" href="#e">FAQ</a></li></ul>
        </div>
      </div>
      <footer className="bg-slate-800 text-white p-4 text-sm">Copyright Acme</footer>
    </div>
  );
}
