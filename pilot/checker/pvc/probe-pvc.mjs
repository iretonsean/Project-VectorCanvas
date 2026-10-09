// pvc/probe-pvc.mjs — client for the `pvc-probe` JSON-lines protocol (PVC_CHECKER_NEEDS.md §2). Same probe interface as BrowserProbe.
import { spawn } from 'node:child_process';
import readline from 'node:readline';

export class PvcProbe {
  constructor(proc) {
    this.proc = proc; this.dialogs = []; this.dialogPolicy = 'dismiss'; this.pending = []; this.diagnostics = [];
    this.rl = readline.createInterface({ input: proc.stdout });
    this.rl.on('line', line => { const p = this.pending.shift(); if (p) { try { p.res(JSON.parse(line)); } catch (e) { p.rej(new Error('bad JSON from pvc-probe: ' + line.slice(0, 120))); } } });
    proc.on('exit', () => { for (const p of this.pending.splice(0)) p.rej(new Error('pvc-probe exited')); });
    this.stderr = ''; proc.stderr.on('data', d => { this.stderr += d; });
  }
  req(obj) {
    return new Promise((res, rej) => { this.pending.push({ res, rej }); this.proc.stdin.write(JSON.stringify(obj) + '\n'); });
  }
  async call(obj) { const r = await this.req(obj); if (!r.ok) throw new Error(r.error || 'pvc-probe error'); return r; }
  async load(opts = {}) { const r = await this.req({ op: 'load', ...opts }); this.diagnostics = r.diagnostics ?? []; this.loadOk = !!r.ok; this.loadError = r.error; return r; }
  async snapshot({ w = 1280, h = 800, scale = 1, dir = 'ltr' } = {}) { const r = await this.call({ op: 'snapshot', viewport: [w, h], text_scale: scale, direction: dir }); return r.snapshot ?? r; }
  async click(id) { await this.call({ op: 'click', id }); }
  async type(id, text, { replace = false } = {}) { await this.call({ op: 'type', id, text, replace }); }
  async typeFocused(text) { await this.call({ op: 'type_focused', text }); }
  async key(key) { const r = await this.call({ op: 'key', key }); return r.focused ?? null; }
  async focused() { return (await this.call({ op: 'focused' })).focused ?? null; }
  async blurAll() { await this.call({ op: 'blur_all' }); }
  async newSession() { await this.call({ op: 'new_session' }); }
  async close() { }
  kill() { try { this.proc.kill(); } catch (e) { /* */ } }
}

/** driver for core/runner.mjs: bin = path of pvc-probe (env PVC_PROBE_BIN), args = extra args (e.g. ['--mock-html', file]) */
export function pvcDriver(bin, file, extraArgs = []) {
  const procs = [];
  const mk = async () => {
    const proc = spawn(bin, [...extraArgs, file], { stdio: ['pipe', 'pipe', 'pipe'] });
    const p = new PvcProbe(proc); procs.push(p);
    const r = await p.load({ fresh_storage: true });
    if (!r.ok) { const e = new Error('LOAD'); e.diagnostics = r.diagnostics ?? []; e.detail = r.error; throw e; }
    return p;
  };
  return { fresh: mk, newSession: async () => procs[procs.length - 1], killAll: () => procs.forEach(p => p.kill()) };
}
