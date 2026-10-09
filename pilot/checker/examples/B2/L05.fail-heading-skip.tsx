// L05: B2 example (React + TypeScript + Tailwind). Mechanical transcription of examples/B1/L05.pass.html.
export default function App() {
  return (
    <div className="[overflow-wrap:anywhere] bg-white text-slate-900 p-4">
<main className="mx-auto max-w-prose py-8">
  <h1 className="text-4xl font-bold mb-2">Why small programs win</h1>
  <p className="text-slate-600 mb-8">By Alex Rivera · 6 min read</p>
  <p className="mb-4">Software has grown heavier every year. Teams add layers on top of layers, and each layer needs its own documentation, its own updates and its own bugs. The result is slow to start and hard to understand.</p>
  <p className="mb-4">A smaller program has fewer places to hide mistakes. When the whole behaviour fits on one screen, a reviewer can read it from top to bottom and know what it does before it runs.</p>
  <blockquote className="border-l-4 border-slate-500 pl-4 my-8 text-2xl italic text-slate-700">Less to read means less to get wrong.</blockquote>
  <p className="mb-4">This matters most for people with older devices. A phone from several years ago still has a fast processor, but it struggles under a heavy runtime. Trimming the runtime gives that power back to the user.</p>
  <h4 className="text-2xl font-semibold mt-8 mb-2">What changes in practice</h4>
  <p className="mb-4">Build times drop first, because there is less to compile. Then review times drop, because there is less to read. Finally support requests drop, because there are fewer surprising interactions.</p>
  <p className="mb-4">None of this is free. Small programs lean on conventions, and conventions must be learned. The learning cost is real, but it is paid once rather than on every change.</p>
  <p className="mb-4">The takeaway is simple: measure the whole cost of a change, from the first request to the last fix, and prefer the path with the smaller total. Size is only one input, but it is the one that compounds.</p>
</main>
</div>
  );
}
