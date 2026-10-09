// L03: B2 example (React + TypeScript + Tailwind). Mechanical transcription of examples/B1/L03.pass.html.
export default function App() {
  return (
    <div className="bg-white text-slate-900">
<main className="max-w-6xl mx-auto p-6 lg:p-12 grid grid-cols-1 lg:grid-cols-2 gap-8 items-center">
  <div className="flex flex-col gap-4">
    <h1 className="text-4xl lg:text-5xl font-extrabold">Build apps that run anywhere</h1>
    <p className="text-lg text-slate-700">One small program, every screen. Describe what you want and ship it.</p>
    <div className="flex flex-wrap gap-3">
      <button className="bg-blue-700 text-white font-semibold rounded-lg px-5 py-3">Get started</button>
      <button className="border-2 border-blue-700 text-blue-800 rounded-lg px-5 py-3">Learn more</button>
    </div>
  </div>
  <svg aria-hidden="true" viewBox="0 0 400 300" className="w-full h-64 lg:h-80 rounded-xl bg-blue-50">
    <circle cx="200" cy="150" r="90" fill="#1d4ed8"/>
    <rect x="60" y="200" width="280" height="30" rx="15" fill="#93c5fd"/>
  </svg>
</main>
</div>
  );
}
