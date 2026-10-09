import { Button } from '@mui/material';

export default function App() {
  return (
    <div className="bg-slate-950 p-4 min-h-screen">
      <div className="w-80 max-w-full rounded-xl bg-slate-800 border border-slate-600 p-6 flex flex-col gap-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-full bg-blue-700 flex items-center justify-center text-white font-bold" aria-hidden="true">JD</div>
          <div>
            <h3 className="text-white font-bold text-sm">Jane Doe</h3>
            <p className="text-slate-300 text-xs">Product Designer</p>
          </div>
        </div>
        <p className="text-slate-200 text-sm leading-relaxed">Building the future of spatial computing via GPU pipelines.</p>
        <Button variant="contained" sx={{ width: 120, bgcolor: '#5eead4', color: '#0f172a', textTransform: 'none', '&:hover': { bgcolor: '#99f6e4' } }} onClick={() => { (window as unknown as { __connected?: boolean }).__connected = true; }}>
          Connect
        </Button>
      </div>
    </div>
  );
}
