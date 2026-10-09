// L02: B2 example (React + TypeScript + Tailwind). Mechanical transcription of examples/B1/L02.pass.html.
export default function App() {
  return (
    <div className="bg-white text-slate-900 p-4">
<main className="max-w-5xl mx-auto">
  <h1 className="text-3xl font-bold text-center mb-8">Pricing</h1>
  <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
    <section className="border border-slate-400 rounded-xl p-6 flex flex-col gap-4">
      <h2 className="text-xl font-semibold">Free</h2>
      <p className="text-3xl font-bold">$0<span className="text-base font-normal text-slate-600"> /month</span></p>
      <ul className="flex flex-col gap-2 text-slate-700">
        <li>1 project</li><li>Community support</li><li>Basic analytics</li>
      </ul>
      <button className="mt-auto bg-slate-800 text-white rounded-lg py-2">Get started</button>
    </section>
    <section className="border-2 border-indigo-700 bg-indigo-50 rounded-xl p-6 flex flex-col gap-4">
      <p className="text-sm font-bold text-indigo-800">Most popular</p>
      <h2 className="text-xl font-semibold">Pro</h2>
      <p className="text-3xl font-bold">$12<span className="text-base font-normal text-slate-600"> /month</span></p>
      <ul className="flex flex-col gap-2 text-slate-700">
        <li>10 projects</li><li>Email support</li><li>Advanced analytics</li><li>50 GB storage</li>
      </ul>
      <button className="mt-auto bg-indigo-700 text-white rounded-lg py-2">Choose Pro</button>
    </section>
    <section className="border border-slate-400 rounded-xl p-6 flex flex-col gap-4">
      <h2 className="text-xl font-semibold">Team</h2>
      <p className="text-3xl font-bold">$29<span className="text-base font-normal text-slate-600"> /month</span></p>
      <ul className="flex flex-col gap-2 text-slate-700">
        <li>Unlimited projects</li><li>Priority support</li><li>Team analytics</li><li>500 GB storage</li>
      </ul>
      <button className="mt-auto bg-slate-800 text-white rounded-lg py-2">Contact sales</button>
    </section>
  </div>
</main>
</div>
  );
}
