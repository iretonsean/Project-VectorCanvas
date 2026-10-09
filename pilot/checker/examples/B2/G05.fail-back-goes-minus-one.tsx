import { useState } from 'react';

const TITLES: Record<number, string> = { 1: 'Step 1: About you', 2: 'Step 2: Pet', 3: 'Step 3: Pet name', 4: 'Step 4: Summary' };

export default function App() {
  const [step, setStep] = useState(1);
  const [pet, setPet] = useState<'yes' | 'no' | null>(null);
  const [hist, setHist] = useState<number[]>([]);
  const next = () => { if (step === 4) return; setHist([...hist, step]); setStep(step === 2 && pet === 'no' ? 4 : step + 1); };
  const back = () => { setStep(step - 1); setHist(hist.slice(0, -1)); };
  return (
    <main className="[overflow-wrap:anywhere] max-w-md mx-auto bg-white text-slate-900 p-4">
      <h1 className="text-2xl font-bold mb-4">{TITLES[step]}</h1>
      <div className="mb-4">
        {step === 1 && <label>Your name <input className="block w-full border border-slate-500 rounded px-2 py-1" /></label>}
        {step === 2 && (
          <fieldset><legend>Do you have a pet?</legend>
            <label><input type="radio" name="pet" checked={pet === 'yes'} onChange={() => setPet('yes')} /> Yes</label>{' '}
            <label><input type="radio" name="pet" checked={pet === 'no'} onChange={() => setPet('no')} /> No</label>
          </fieldset>
        )}
        {step === 3 && <label>Pet name <input className="block w-full border border-slate-500 rounded px-2 py-1" /></label>}
        {step === 4 && <p>All done.</p>}
      </div>
      <div className="flex gap-2">
        {hist.length > 0 && <button onClick={back} className="border border-slate-700 rounded px-3 py-1">Back</button>}
        <button onClick={next} className="bg-blue-700 text-white rounded px-3 py-1">{step === 4 ? 'Finish' : 'Next'}</button>
      </div>
    </main>
  );
}
