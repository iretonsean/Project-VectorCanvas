#!/usr/bin/env python3
"""summarize.py <cell_dir>  — builds summary.json + summary.md from raw trace_rep*.csv / meta_rep*.json / runmeta_rep*.json
Implements METHOD.md §2, §6, §7, §8, §10, §13 exactly. Python stdlib + numpy."""
import csv, glob, json, os, sys
import numpy as np

MEASURED = {  # METHOD §2.1
    'ptr-move': ['ptr_move'], 'tab': ['key_tab'], 'type-latin': ['key_char'],
    'ime-zh': ['ime_preedit', 'ime_commit'], 'toggle-show': ['ptr_down', 'ptr_up'], 'checkbox': ['ptr_down', 'ptr_up'],
}
WARMUP = 10
BUDGET_FRAME_MS = {'tier0c-vello_cpu': 33.0, 'gl-llvmpipe': 16.6, 'chromium141-webgl2-swiftshader': 16.6}
BUDGET_FIRST_MS = {'tier0c-vello_cpu': 1500.0, 'gl-llvmpipe': 1000.0, 'chromium141-webgl2-swiftshader': 1000.0}
RSS_BUDGET_KB = 153600  # 150 MiB (METHOD §8); 150_000 kB also printed
LABEL = {
    'tier0c-vello_cpu': 'host=container-xeon4 path=tier0c-vello_cpu',
    'gl-llvmpipe': 'host=container-xeon4 path=gl-llvmpipe',
    'chromium141-webgl2-swiftshader': 'host=container-xeon4 path=chromium141-webgl2-swiftshader',
}
STAGES = ['update_ms', 'layout_ms', 'scene_ms', 'a11y_ms', 'mirror_ms', 'render_ms', 'total_ms', 'completion_ms']

def fnum(x):
    try:
        return float(x) if x not in ('', None) else None
    except ValueError:
        return None

def pct(a, q):
    return float(np.percentile(np.asarray(a, dtype=float), q, method='linear')) if len(a) else None

def stats(a):
    if not len(a): return None
    return {'n': len(a), 'p50': pct(a, 50), 'p95': pct(a, 95), 'p99': pct(a, 99), 'max': float(np.max(a)), 'mean': float(np.mean(a))}

def read_csv(p):
    with open(p, newline='') as f:
        return list(csv.DictReader(f))

def main(d):
    runmetas = sorted(glob.glob(f'{d}/runmeta_rep*.json'))
    reps = []
    for rm in runmetas:
        k = rm.split('rep')[-1].split('.')[0]
        meta = json.load(open(rm))
        rows = read_csv(f'{d}/trace_rep{k}.csv') if os.path.exists(f'{d}/trace_rep{k}.csv') else []
        reps.append((k, meta, rows))
    if not reps:
        print('no reps in', d); return
    m0 = reps[0][1]
    path, specimen, scenario = m0['path'], m0['specimen'], m0['scenario']
    S = {'label': LABEL[path], 'path': path, 'specimen': specimen, 'scenario': scenario,
         'variant': m0.get('variant'), 'mirror': m0.get('mirror'), 'mode': m0.get('mode'), 'atspi': m0.get('atspi'),
         'constraint_solve': 'general constraint solve excluded (D-4): layout is the direct sugar solver',
         'not_tier1_device': 'NOT a Tier 1 device result; no phone or Chromebook was measured',
         'reps': len(reps), 'artifacts_sha256': m0.get('artifacts_sha256'), 'cmd': m0.get('cmd'),
         'toolchain': m0.get('toolchain'), 'date_utc': m0.get('date_utc'),
         'loadavg_start': {k: m['loadavg_start'] for k, m, _ in reps},
         'noisy_reps': [k for k, m, _ in reps if m.get('noisy')],
         'budget_frame_ms': BUDGET_FRAME_MS[path]}
    budget = BUDGET_FRAME_MS[path]

    if scenario in MEASURED:
        classes = MEASURED[scenario]
        pool = {s: [] for s in STAGES}; per_rep_p95 = []; warm_max = []; sent = 0; got = 0
        by_event = {}
        for k, m, rows in reps:
            meas = [r for r in rows if r['event'] in classes]
            sent += m.get('events_sent', 0); got += len(meas)
            warm, keep = meas[:WARMUP], meas[WARMUP:]
            if warm: warm_max.append(max(fnum(r['completion_ms']) or fnum(r['total_ms']) or 0 for r in warm))
            prim = []
            for r in keep:
                for s in STAGES:
                    v = fnum(r.get(s)); 
                    if v is not None: pool[s].append(v)
                by_event.setdefault(r['event'], []).append(fnum(r['completion_ms']) if fnum(r['completion_ms']) is not None else fnum(r['total_ms']))
                prim.append(fnum(r['completion_ms']) if fnum(r['completion_ms']) is not None else fnum(r['total_ms']))
            if prim: per_rep_p95.append(pct(prim, 95))
        have_completion = len(pool['completion_ms']) > 0
        primary_name = 'completion_ms' if have_completion else 'total_ms'
        S['primary_metric'] = primary_name
        S['frames_retained'] = len(pool['total_ms']); S['warmup_discarded_per_rep'] = WARMUP
        S['events_sent'] = sent; S['frames_recorded_measured_classes'] = got
        S['frame_per_event'] = (got / sent) if sent else None
        S['coalesced_flag'] = bool(sent and got / sent < 0.9)
        S['stages'] = {s: stats(pool[s]) for s in STAGES if pool[s]}
        S['per_event_class_primary'] = {e: stats([x for x in v if x is not None]) for e, v in by_event.items()}
        S['per_rep_p95_primary'] = per_rep_p95
        S['warmup_max_ms'] = {'max_over_reps': max(warm_max) if warm_max else None}
        prim_stats = S['stages'].get(primary_name)
        S['primary'] = prim_stats
        if prim_stats:
            S['pass_p95'] = prim_stats['p95'] <= budget
            S['pass_strict_max'] = prim_stats['max'] <= budget
            if path == 'gl-llvmpipe':
                S['pass_p95_vs_33ms_hardware_class'] = prim_stats['p95'] <= 33.0
        if pool['mirror_ms'] and pool['total_ms']:
            # Q3: absolute and share (ratio of sums, and per-frame ratio p50/p95) over paired frames
            mi = []; to = []
            for k, m, rows in reps:
                meas = [r for r in rows if r['event'] in classes][WARMUP:]
                for r in meas:
                    a, b = fnum(r['mirror_ms']), fnum(r['total_ms'])
                    if a is not None and b: mi.append(a); to.append(b)
            ratios = [a / b for a, b in zip(mi, to)]
            S['mirror'] = {'absolute_ms': stats(mi), 'share_of_total_ratio_of_sums': float(sum(mi) / sum(to)),
                           'share_per_frame_p50': pct(ratios, 50), 'share_per_frame_p95': pct(ratios, 95)}
    elif scenario == 'idle':
        per = {}
        for k, m, rows in reps:
            w0, w1 = m['idle']['window_start_ns'], m['idle']['window_end_ns']
            rows_in = [r for r in rows if w0 <= int(r['t_ns']) < w1]
            gl = m['idle'].get('gl_calls_in_window')
            per[k] = {'trace_rows_in_window': len(rows_in), 'gl_calls_in_window': gl,
                      'cpu_ticks_in_window': m['idle'].get('cpu_ticks'), 'vol_ctxt_switches': m['idle'].get('vol_ctxt'),
                      'frames_reported': max(len(rows_in), (gl or 0) and 1 or 0)}
        S['idle_per_rep'] = per
        S['idle_frames_max_over_reps'] = max(v['frames_reported'] for v in per.values())
        S['idle_fps'] = S['idle_frames_max_over_reps'] / 5.0
        S['pass_idle_zero'] = all(v['trace_rows_in_window'] == 0 and not v['gl_calls_in_window'] for v in per.values())
        S['idle_state'] = m0.get('idle_state', 'idle-initial')
    elif scenario == 'first-frame':
        ff = []
        for k, m, rows in reps:
            if not rows: continue
            r0 = rows[0]
            comp = fnum(r0.get('completion_ms')); tot = fnum(r0.get('total_ms'))
            end_ms = comp if comp is not None else tot
            if m['path'] == 'chromium141-webgl2-swiftshader':
                ff.append(int(r0['t_ns']) / 1e6 + end_ms)
            else:
                ff.append((int(r0['t_ns']) - m['t_spawn_ns']) / 1e6 + end_ms)
        S['first_frame_ms_each_rep'] = ff
        S['first_frame_ms'] = stats(ff)
        S['first_frame_budget_ms'] = BUDGET_FIRST_MS[path]
        if ff:
            S['pass_first_frame_max_lt_budget'] = max(ff) < BUDGET_FIRST_MS[path]
        S['crosscheck_goto_to_first_row_ms'] = [m.get('goto_to_first_row_ms') for _, m, _ in reps]
    elif scenario == 'rss':
        rs = [m['rss'] for _, m, _ in reps if m.get('rss')]
        S['rss_per_rep'] = rs
        key = 'vmhwm_kb' if path != 'chromium141-webgl2-swiftshader' else 'web_rss_renderer_gpu_sum_kb'
        vals = [r.get(key) for r in rs if r.get(key) is not None]
        if vals:
            S['rss_headline_key'] = key
            S['rss_max_kb'] = max(vals); S['rss_median_kb'] = float(np.median(vals))
            S['pass_rss_153600kb'] = max(vals) <= RSS_BUDGET_KB
            S['pass_rss_150000kb'] = max(vals) <= 150000
    json.dump(S, open(f'{d}/summary.json', 'w'), indent=1)
    write_md(d, S)

def fm(x, nd=2):
    return '—' if x is None else f'{x:.{nd}f}'

def write_md(d, S):
    L = [f"# {S['path']} / {S['specimen']} / {S['scenario']}", '',
         f"**Label:** `{S['label']}`" + (f" mode={S['mode']}" if S.get('mode') else '') + (f" variant={S['variant']}" if S.get('variant') else '') + (f" mirror={S['mirror']}" if S.get('mirror') else '') + (f" atspi={S['atspi']}" if S.get('atspi') else ''), '',
         f"- {S['constraint_solve']}", f"- {S['not_tier1_device']}",
         f"- reps: {S['reps']}; loadavg at start of each rep: {S['loadavg_start']}; noisy reps: {S['noisy_reps'] or 'none'}",
         f"- cmd: `{S.get('cmd')}`", f"- toolchain: {S.get('toolchain')}; date: {S.get('date_utc')}", f"- artifacts sha256: {S.get('artifacts_sha256')}", '']
    if 'primary' in S:
        p = S['primary']
        L += [f"## Frame time (primary metric `{S['primary_metric']}`; budget {S['budget_frame_ms']} ms; METHOD §13)", '',
              f"n={p['n']} retained frames ({S['warmup_discarded_per_rep']} warm-up frames per rep discarded; warm-up max over reps {fm(S['warmup_max_ms']['max_over_reps'])} ms)", '',
              '| metric | p50 | p95 | p99 | max |', '|---|---|---|---|---|']
        for s, st in S['stages'].items():
            L.append(f"| {s} | {fm(st['p50'],3)} | {fm(st['p95'],3)} | {fm(st['p99'],3)} | {fm(st['max'],3)} |")
        L += ['', f"- **pass (p95 ≤ {S['budget_frame_ms']} ms): {'PASS' if S.get('pass_p95') else 'FAIL'}**; strict (max ≤ budget): {'PASS' if S.get('pass_strict_max') else 'FAIL'}"]
        if 'pass_p95_vs_33ms_hardware_class' in S:
            L.append(f"- informational vs Tier 0 hardware-class budget 33 ms: p95 {'PASS' if S['pass_p95_vs_33ms_hardware_class'] else 'FAIL'}")
        L += [f"- per-rep p95 ({S['primary_metric']}): {[round(x,3) for x in S['per_rep_p95_primary']]}",
              f"- events sent {S['events_sent']}, frames recorded {S['frames_recorded_measured_classes']}, frame_per_event {fm(S['frame_per_event'])}" + (' **⚠coalesced**' if S['coalesced_flag'] else '')]
        if 'mirror' in S:
            m = S['mirror']; a = m['absolute_ms']
            L += ['', '## Q3 mirror cost', f"- mirror_ms absolute: p50 {fm(a['p50'],3)} p95 {fm(a['p95'],3)} p99 {fm(a['p99'],3)} max {fm(a['max'],3)} ms",
                  f"- share of web frame (Σmirror/Σtotal): {fm(100*m['share_of_total_ratio_of_sums'],1)} %; per-frame share p50 {fm(100*m['share_per_frame_p50'],1)} % p95 {fm(100*m['share_per_frame_p95'],1)} %"]
    if 'idle_per_rep' in S:
        L += [f"## Idle ({S['idle_state']}; 1 s settle then 5 s, no input)", '', '| rep | trace rows in window | GL draw/clear calls in window | CPU ticks | vol ctxt switches |', '|---|---|---|---|---|']
        for k, v in S['idle_per_rep'].items():
            L.append(f"| {k} | {v['trace_rows_in_window']} | {v['gl_calls_in_window']} | {v['cpu_ticks_in_window']} | {v['vol_ctxt_switches']} |")
        L += ['', f"- idle FPS (max over reps) = {S['idle_fps']}; **{'PASS (exactly 0 in every rep)' if S['pass_idle_zero'] else 'FAIL'}**"]
    if 'first_frame_ms' in S:
        f = S['first_frame_ms']
        L += ['## First frame', '', f"- per rep (ms): {[round(x,1) for x in S['first_frame_ms_each_rep']]}", f"- median {fm(f['p50'],1)} ms, max {fm(f['max'],1)} ms; budget < {S['first_frame_budget_ms']} ms → **{'PASS' if S.get('pass_first_frame_max_lt_budget') else 'FAIL'}** (max over reps)"]
    if 'rss_per_rep' in S:
        L += ['## RSS', '', f"- headline `{S.get('rss_headline_key')}`: max {S.get('rss_max_kb')} kB, median {S.get('rss_median_kb')} kB; budget 153600 kB (150 MiB) → **{'PASS' if S.get('pass_rss_153600kb') else 'FAIL'}**; vs 150000 kB → {'PASS' if S.get('pass_rss_150000kb') else 'FAIL'}", '', '```', json.dumps(S['rss_per_rep'], indent=1), '```']
    open(f'{d}/summary.md', 'w').write('\n'.join(L) + '\n')

if __name__ == '__main__':
    main(sys.argv[1].rstrip('/'))
