// L07: B2 example (React + TypeScript + Tailwind). Mechanical transcription of examples/B1/L07.pass.html.
export default function App() {
  return (
    <div className="[overflow-wrap:anywhere] bg-slate-50 text-slate-900">
<header className="bg-white border-b border-slate-300 p-3 flex items-center gap-3">
  <span className="font-bold">Acme</span>
  <input type="search" aria-label="Search" placeholder="Search" className="flex-1 min-w-0 border border-slate-500 rounded px-3 py-1" />
  <button aria-label="Account menu" className="rounded-full w-9 h-9 bg-blue-700 text-white text-sm shrink-0">JD</button>
</header>
<main className="p-4 max-w-6xl mx-auto">
  <h1 className="text-xl font-bold mb-4">Overview</h1>
  <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
    <div className="rounded-lg border border-slate-300 bg-white p-4 min-w-0"><p className="text-sm text-slate-600">Users</p><p className="text-2xl font-bold">1,284</p></div>
    <div className="rounded-lg border border-slate-300 bg-white p-4 min-w-0"><p className="text-sm text-slate-600">Uptime</p><p className="text-2xl font-bold">92%</p></div>
    <div className="rounded-lg border border-slate-300 bg-white p-4 min-w-0"><p className="text-sm text-slate-600">Revenue</p><p className="text-2xl font-bold">$4.2k</p></div>
    <div className="rounded-lg border border-slate-300 bg-white p-4 min-w-0"><p className="text-sm text-slate-600">Tickets</p><p className="text-2xl font-bold">37</p></div>
  </div>
  <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mt-4">
    <section className="rounded-lg border border-slate-300 bg-white p-4 h-64"><h2 className="font-semibold">Traffic</h2><p className="text-slate-600">Recent visits by day.</p></section>
    <section className="rounded-lg border border-slate-300 bg-white p-4 h-64"><h2 className="font-semibold">Activity</h2><p className="text-slate-600">Latest events across the team.</p></section>
  </div>
</main>
</div>
  );
}
