import { useState, type FormEvent } from 'react';

type Task = { id: number; text: string; done: boolean };
type Filter = 'All' | 'Active' | 'Done';

export default function App() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [text, setText] = useState('');
  const [filter, setFilter] = useState<Filter>('All');
  const [id, setId] = useState(0);
  const left = tasks.filter((t) => !t.done).length;
  const shown = tasks.filter((t) => filter === 'All' || (filter === 'Active' ? !t.done : t.done));
  const add = (e: FormEvent) => { e.preventDefault(); if (text.trim()) { setTasks([...tasks, { id: id + 1, text: text.trim(), done: false }]); setId(id + 1); setText(''); } };
  return (
    <main className="[overflow-wrap:anywhere] max-w-md mx-auto bg-white text-slate-900 p-4">
      <h1 className="text-2xl font-bold mb-4">To-do</h1>
      <form onSubmit={add} className="flex gap-2 mb-4">
        <label htmlFor="new" className="sr-only">New task</label>
        <input id="new" value={text} onChange={(e) => setText(e.target.value)} className="flex-1 min-w-0 border border-slate-500 rounded px-2 py-1" placeholder="New task" />
        <button className="bg-blue-700 text-white rounded px-3 py-1">Add</button>
      </form>
      <div className="flex gap-2 mb-2">
        {(['All', 'Active', 'Done'] as Filter[]).map((f) => (
          <button key={f} aria-pressed={filter === f} onClick={() => setFilter(f)} className="border border-slate-700 rounded px-2 py-1">{f}</button>
        ))}
      </div>
      <ul className="flex flex-col gap-1">
        {shown.map((t) => (
          <li key={t.id} className="flex items-center gap-2">
            <label className="flex items-center gap-2 flex-1">
              <input type="checkbox" checked={t.done} onChange={() => setTasks(tasks.map((x) => (x.id === t.id ? { ...x, done: !x.done } : x)))} />
              <span>{t.text}</span>
            </label>
            <button aria-label={'Delete ' + t.text} className="border border-slate-700 rounded px-2 py-0.5" onClick={() => setTasks(tasks.filter((x) => x.id !== t.id))}>Delete</button>
          </li>
        ))}
      </ul>
      <p className="mt-3 text-slate-700">{left + (left === 1 ? ' item left' : ' items left')}</p>
    </main>
  );
}
