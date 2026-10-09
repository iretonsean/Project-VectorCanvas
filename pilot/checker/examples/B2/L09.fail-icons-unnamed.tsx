// L09: B2 example (React + TypeScript + Tailwind). Mechanical transcription of examples/B1/L09.pass.html.
export default function App() {
  return (
    <div className="bg-white text-slate-900">
<footer className="bg-slate-900 text-slate-100 p-6">
  <div className="max-w-6xl mx-auto grid grid-cols-1 md:grid-cols-4 gap-6">
    <div><h3 className="font-semibold mb-2">Product</h3><ul className="flex flex-col gap-1"><li><a className="underline" href="#1">Features</a></li><li><a className="underline" href="#2">Pricing</a></li><li><a className="underline" href="#3">Changelog</a></li></ul></div>
    <div><h3 className="font-semibold mb-2">Company</h3><ul className="flex flex-col gap-1"><li><a className="underline" href="#4">About</a></li><li><a className="underline" href="#5">Careers</a></li><li><a className="underline" href="#6">Press</a></li></ul></div>
    <div><h3 className="font-semibold mb-2">Resources</h3><ul className="flex flex-col gap-1"><li><a className="underline" href="#7">Docs</a></li><li><a className="underline" href="#8">Blog</a></li><li><a className="underline" href="#9">Support</a></li></ul></div>
    <div><h3 className="font-semibold mb-2">Legal</h3><ul className="flex flex-col gap-1"><li><a className="underline" href="#10">Privacy</a></li><li><a className="underline" href="#11">Terms</a></li><li><a className="underline" href="#12">Cookies</a></li></ul></div>
  </div>
  <form className="max-w-6xl mx-auto mt-6 flex flex-wrap items-center gap-2" onSubmit={(e) => e.preventDefault()}>
    <label htmlFor="nl-email">Email</label>
    <input id="nl-email" type="email" className="border border-slate-300 bg-white text-slate-900 rounded px-2 py-1 min-w-0" />
    <button className="bg-teal-300 text-slate-900 rounded px-3 py-1">Subscribe</button>
  </form>
  <div className="max-w-6xl mx-auto mt-6 flex items-center gap-4">
    <a href="#tw" className="inline-block"><svg aria-hidden="true" className="w-6 h-6" viewBox="0 0 24 24" fill="currentColor"><circle cx="12" cy="12" r="10"/></svg></a>
    <a href="#gh" className="inline-block"><svg aria-hidden="true" className="w-6 h-6" viewBox="0 0 24 24" fill="currentColor"><rect x="3" y="3" width="18" height="18" rx="3"/></svg></a>
  </div>
  <p className="max-w-6xl mx-auto mt-6 text-sm text-slate-300">© 2026 Acme Inc. All rights reserved.</p>
</footer>
</div>
  );
}
