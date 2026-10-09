import { useState } from 'react';

export default function App() {
  const [notes, setNotes] = useState<{ id: number; label: string }[]>([]);
  const [n, setN] = useState(0);
  const [dismissed, setDismissed] = useState(false);
  return (
    <main className="[overflow-wrap:anywhere] max-w-md mx-auto bg-white text-slate-900 p-4">
      <h1 className="text-2xl font-bold mb-4">Notes</h1>
      <button className="bg-blue-700 text-white rounded px-3 py-1" onClick={() => { setN(n + 1); setNotes([...notes, { id: n + 1, label: 'Note ' + (n + 1) }]); }}>Add note</button>
      <ul className="my-4 flex flex-col gap-2">
        {notes.map((x) => (
          <li key={x.id} className="flex items-center gap-2">
            <span>{x.label}</span>
            <button className="border border-slate-700 rounded px-2 py-0.5" aria-label={'Remove ' + x.label} onClick={() => setNotes(notes.filter((y) => y.id !== x.id))}>Remove</button>
          </li>
        ))}
      </ul>
      {notes.length === 0 && !dismissed && (
        <div className="rounded border border-slate-400 bg-slate-100 p-3">
          <p>No notes yet. Add your first note.</p>
          <button className="mt-2 border border-slate-700 rounded px-3 py-1" onClick={() => setDismissed(true)}>Dismiss</button>
        </div>
      )}
    </main>
  );
}
