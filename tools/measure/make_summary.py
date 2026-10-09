#!/usr/bin/env python3
"""make_summary.py — builds measurements/session2/SUMMARY.md from every cell's summary.json (+ not_measurable.json). No hand-typed numbers."""
import glob, json, os, sys
root = os.path.abspath(os.path.join(os.path.dirname(__file__), '../../measurements/session2'))
PATHS = ['tier0c-vello_cpu', 'gl-llvmpipe', 'chromium141-webgl2-swiftshader']
SCEN = {'L01': ['first-frame', 'idle', 'ptr-move', 'rss'], 'F01': ['first-frame', 'idle', 'tab', 'type-latin', 'ime-zh', 'toggle-show', 'checkbox', 'rss']}
NA = {('L01', s): 'n/a: L01 has no text field/toggle/checkbox (METHOD §2.1)' for s in ['tab', 'type-latin', 'ime-zh', 'toggle-show', 'checkbox']}
NA[('F01', 'ptr-move')] = 'n/a: §4.4.1 names pointer move over the L01 button; not run on F01 (METHOD §2.1)'
nm = json.load(open(f'{root}/not_measurable.json')) if os.path.exists(f'{root}/not_measurable.json') else []
def f(x, n=2): return '—' if x is None else f'{x:.{n}f}'
def pf(b): return '—' if b is None else ('PASS' if b else 'FAIL')
L = ['# SUMMARY — Session 2 measurements (dev-measure)', '',
     '**Every number is `host=container-xeon4` (4 vCPU Xeon @ 2.10 GHz, no GPU) on the path named in its row. NOT Tier 1 device results; no phone or Chromebook was measured. General constraint solve excluded (D-4).**',
     'Method: [`METHOD.md`](../../METHOD.md) (written before any number). Pass rule (decision, METHOD §13): p95 of `completion_ms` (web `raf`: `total_ms`) ≤ budget; strict = max ≤ budget. Budgets: 16.6 ms Tier-1 paths, 33 ms Tier 0, 150 MiB, idle exactly 0, first frame < 1 s / < 1.5 s. A pass means "passed on this container path", never "meets the Tier 1 budget". Web `total_ms` is work-only (waiting excluded), native is one continuous span (METHOD §2.2).', '',
     '| path | variant | specimen | scenario | metric | n | p50 ms | p95 ms | p99 ms | max ms | pass (p95) | strict (max) | idle frames/5 s | first frame ms (median / max) | RSS kB (max) | mirror p95 ms | mirror share | file |', '|' + '---|' * 18]
for p in PATHS:
    for sp in ['L01', 'F01']:
        for sc in SCEN[sp]:
            dirs = sorted(glob.glob(f'{root}/{p}/{sp}/{sc}/summary.json') + glob.glob(f'{root}/{p}/{sp}/{sc}/*/summary.json'))
            if not dirs:
                why = NA.get((sp, sc)) or next((x['reason'] for x in nm if x['path'] == p and x['specimen'] == sp and x['scenario'] == sc), None) or 'NOT YET MEASURED'
                L.append(f'| {p} | | {sp} | {sc} | | | | | | | | | | | | | | {why} |'); continue
            for d in dirs:
                S = json.load(open(d)); rel = os.path.relpath(d.replace('summary.json', 'summary.md'), root)
                var = '/'.join(x for x in [S.get('variant'), ('mirror-' + S['mirror']) if S.get('mirror') else None, ('atspi-' + str(S.get('atspi'))) if S.get('atspi') else None] if x)
                noisy = ' ⚠noisy' if S.get('noisy_reps') else ''
                pr = S.get('primary'); ff = S.get('first_frame_ms'); mi = S.get('mirror')
                idle = S.get('idle_frames_max_over_reps')
                rss = S.get('rss_max_kb')
                L.append(f"| {p} | {var} | {sp} | {sc}{noisy} | {S.get('primary_metric','')} | {pr['n'] if pr else ''} | {f(pr['p50'],3) if pr else ''} | {f(pr['p95'],3) if pr else ''} | {f(pr['p99'],3) if pr else ''} | {f(pr['max'],3) if pr else ''} | {pf(S.get('pass_p95')) if pr else (pf(S.get('pass_idle_zero')) if idle is not None else (pf(S.get('pass_first_frame_max_lt_budget')) if ff else (pf(S.get('pass_rss_153600kb')) if rss else '')))} | {pf(S.get('pass_strict_max')) if pr else ''} | {idle if idle is not None else ''} | {(f(ff['p50'],0)+' / '+f(ff['max'],0)) if ff else ''} | {rss if rss else ''} | {f(mi['absolute_ms']['p95'],3) if mi else ''} | {(f(100*mi['share_of_total_ratio_of_sums'],1)+' %') if mi else ''} | [summary]({rel}) |")
L += ['', 'Cells marked `NOT YET MEASURED` / `n/a` / `not measurable here` are explicit: nothing is hidden. Q3 (mirror): `mirror_ms` absolute and as share of the web frame, per variant (`raf` = rAF-callback work; `sync` = plus 1×1 `readPixels`).']
open(f'{root}/SUMMARY.md', 'w').write('\n'.join(L) + '\n')
print('wrote', f'{root}/SUMMARY.md')
