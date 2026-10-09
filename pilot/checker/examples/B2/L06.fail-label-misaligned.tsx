// L06: B2 example (React + TypeScript + Tailwind). Mechanical transcription of examples/B1/L06.pass.html.
export default function App() {
  return (
    <div className="[overflow-wrap:anywhere] bg-white text-slate-900 p-4">
<main className="max-w-xl mx-auto">
  <h1 className="text-2xl font-bold mb-6">Settings</h1>
  <section className="mb-8">
    <h2 className="text-lg font-semibold mb-2">Account</h2>
    <div className="flex items-center justify-between py-2"><label htmlFor="s-name">Display name</label><input id="s-name" type="text" className="border border-slate-500 rounded px-2 py-1 w-40 max-w-[45%]" /></div>
    <div className="flex items-center justify-between py-2"><label htmlFor="s-mail" className="ml-8">Email</label><input id="s-mail" type="text" className="border border-slate-500 rounded px-2 py-1 w-40 max-w-[45%]" /></div>
  </section>
  <section className="mb-8">
    <h2 className="text-lg font-semibold mb-2">Notifications</h2>
    <div className="flex items-center justify-between py-2"><label htmlFor="s-n1">Email notifications</label><input id="s-n1" type="checkbox" className="h-5 w-5" /></div>
    <div className="flex items-center justify-between py-2"><label htmlFor="s-n2">Push notifications</label><input id="s-n2" type="checkbox" className="h-5 w-5" /></div>
  </section>
  <section>
    <h2 className="text-lg font-semibold mb-2">Appearance</h2>
    <div className="flex items-center justify-between py-2"><label htmlFor="s-theme">Theme</label>
      <select id="s-theme" className="border border-slate-500 rounded px-2 py-1 w-40 max-w-[45%]"><option>Light</option><option>Dark</option></select></div>
    <div className="flex items-center justify-between py-2"><label htmlFor="s-dense">Compact layout</label><input id="s-dense" type="checkbox" className="h-5 w-5" /></div>
  </section>
</main>
</div>
  );
}
