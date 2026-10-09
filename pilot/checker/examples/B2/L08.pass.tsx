// L08: B2 example (React + TypeScript + Tailwind). Mechanical transcription of examples/B1/L08.pass.html.
export default function App() {
  return (
    <div className="[overflow-wrap:anywhere] bg-slate-100 text-slate-900 p-4">
<article className="max-w-md rounded-xl border border-slate-400 bg-white p-4 flex flex-wrap gap-4">
  <div className="w-16 shrink-0 rounded-lg bg-rose-700 text-white text-center py-2">
    <div className="text-xs font-bold uppercase">Oct</div>
    <div className="text-3xl font-bold">12</div>
  </div>
  <div className="flex flex-col gap-2 basis-48 min-w-0 flex-1">
    <h2 className="text-xl font-bold">Autumn Jazz Night</h2>
    <p className="flex items-center gap-1 text-slate-700">
      <svg aria-hidden="true" className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="currentColor"><path d="M12 2a7 7 0 0 0-7 7c0 5 7 13 7 13s7-8 7-13a7 7 0 0 0-7-7z"/></svg>
      <span>Riverside Hall</span>
    </p>
    <a href="#cal" className="text-blue-800 underline">Add to calendar</a>
  </div>
</article>
</div>
  );
}
