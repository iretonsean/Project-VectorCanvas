#!/usr/bin/env python3
"""tools/measure/native_run.py — one repetition of one (specimen, scenario) on a native path (METHOD.md §2.2 native, §6, §7, §8, §9).
Paths: tier0c-vello_cpu (pvc-native --renderer cpu, windowed on Xvfb) | gl-llvmpipe (pvc-native --renderer gl).
Expects DISPLAY / DBUS_SESSION_BUS_ADDRESS / XMODIFIERS exported (run.sh sources dev-native's env_up.sh with a private run dir and display).
Must run under `flock -x CPU_LOCK` (run.sh does that).  Writes <out>/trace_rep<K>.csv and runmeta_rep<K>.json."""
import argparse, csv, hashlib, json, os, signal, subprocess, sys, time

ap = argparse.ArgumentParser()
ap.add_argument('--path', required=True, choices=['tier0c-vello_cpu', 'gl-llvmpipe'])
ap.add_argument('--bin', required=True)
ap.add_argument('--artifact', required=True)
ap.add_argument('--specimen', required=True)
ap.add_argument('--scenario', required=True)
ap.add_argument('--out', required=True)
ap.add_argument('--rep', default='1')
ap.add_argument('--cmd', default=' '.join(sys.argv))
a = ap.parse_args()
os.makedirs(a.out, exist_ok=True)
GAP = 0.05
renderer = 'cpu' if a.path == 'tier0c-vello_cpu' else 'gl'
locale = 'zh-CN' if a.scenario == 'ime-zh' else 'en'
rep = a.rep

def read_stat():
    l = [int(x) for x in open('/proc/stat').readline().split()[1:]]
    return l[3] + (l[4] if len(l) > 4 else 0), sum(l)
def idle_probe(sec=3.0):
    i0, t0 = read_stat(); time.sleep(sec); i1, t1 = read_stat(); return round(100 * (i1 - i0) / (t1 - t0), 1)

probe = idle_probe(); noisy = False
for _ in range(6):
    if probe >= 90: break
    time.sleep(20); probe = idle_probe()
if probe < 90: noisy = True
loadavg = open('/proc/loadavg').read().strip()

def sha(p):
    h = hashlib.sha256()
    with open(p, 'rb') as f:
        for b in iter(lambda: f.read(1 << 20), b''): h.update(b)
    return h.hexdigest()

trace = os.path.join(a.out, f'trace_rep{rep}.csv'); metaf = os.path.join(a.out, f'meta_rep{rep}.json'); rectf = os.path.join(a.out, f'rects_rep{rep}.json')
for f in (trace, metaf, rectf):
    if os.path.exists(f): os.remove(f)
env = dict(os.environ, PVC_TRACE=trace, PVC_META=metaf, PVC_RECTS=rectf, PVC_SCENARIO=a.scenario)
host_log = open(os.path.join(a.out, f'host_rep{rep}.log'), 'w')
t_spawn_ns = time.time_ns()
proc = subprocess.Popen([a.bin, a.artifact, '--locale', locale, '--renderer', renderer], env=env, stdout=host_log, stderr=subprocess.STDOUT)
pid = proc.pid
for _ in range(600):
    if os.path.exists(rectf) and os.path.getsize(rectf) > 2: break
    if proc.poll() is not None: sys.exit('host exited early; see host log')
    time.sleep(0.1)
else:
    proc.kill(); sys.exit('host did not publish rects (no first frame)')
R = json.load(open(rectf))
centre = lambda r: (int(r[0] + r[2] / 2), int(r[1] + r[3] / 2))

def nrows():
    try:
        with open(trace) as f: return max(0, sum(1 for _ in f) - 1)
    except FileNotFoundError: return 0
def settle(quiet=0.3):
    last, since = -1, time.time()
    while True:
        n = nrows()
        if n != last: last, since = n, time.time()
        elif time.time() - since >= quiet: return n
        time.sleep(0.02)

def xdo(lines):
    p = subprocess.run(['xdotool', '-'], input='\n'.join(lines) + '\n', text=True, capture_output=True)
    if p.returncode != 0: raise RuntimeError('xdotool failed: ' + p.stderr)
def focus():
    wid = subprocess.run(['xdotool', 'search', '--name', '^PVC$'], capture_output=True, text=True).stdout.split()
    if wid: subprocess.run(['xdotool', 'windowfocus', wid[0]]); time.sleep(0.2)
def mv(r): x, y = centre(r); return f'mousemove {x} {y}'
sleep = lambda s: f'sleep {s}'
KEY = {' ': 'space'}
sent = 0
idle = {}
def cpu_ticks():
    f = open(f'/proc/{pid}/stat').read().rsplit(')', 1)[1].split(); return int(f[11]) + int(f[12])
def vol_ctxt():
    for l in open(f'/proc/{pid}/status'):
        if l.startswith('voluntary_ctxt_switches'): return int(l.split()[1])

focus(); settle(0.5)

def S_ptr_move():
    global sent
    inn, out = centre(R['connect_button']), centre(R['bio_text'])
    xdo([f'mousemove {inn[0]} {inn[1]}', sleep(0.3)]); settle()
    L = []
    for i in range(200):
        t = out if i % 2 == 0 else inn
        L += [f'mousemove {t[0]} {t[1]}', sleep(GAP)]; sent += 1
    xdo(L); settle()
def S_tab():
    global sent
    xdo([mv([20, 20, 0, 0]), 'click 1', sleep(0.3)]); settle()
    xdo(sum([['key Tab', sleep(GAP)] for _ in range(60)], [])); sent += 60; settle()
def S_type_latin():
    global sent
    xdo([mv(R['name']), 'click 1', sleep(0.3)]); settle()
    for _ in range(5):
        L = []
        for ch in 'Ada Lovelace Byron 1': L += [f'key {KEY.get(ch, ch)}', sleep(GAP)]; sent += 1
        xdo(L); settle()
        xdo(['key ctrl+a', sleep(0.05), 'key Delete', sleep(0.05)] + ['key BackSpace', sleep(0.01)] * 25); settle()
def S_ime():
    global sent
    xdo([mv(R['name']), 'click 1', sleep(0.3)]); settle()
    xdo(['key ctrl+space', sleep(0.5)]); settle()   # fcitx5 trigger: keyboard-us -> pinyin (dev-native's setup)
    for _ in range(20):
        L = []
        for c in 'nihao': L += [f'key {c}', sleep(GAP)]; sent += 1
        L += ['key space', sleep(GAP)]; sent += 1
        xdo(L); settle()
        xdo(['key BackSpace', sleep(0.05), 'key BackSpace', sleep(0.05)]); settle()
def clicks(r):
    global sent
    xdo([mv(r), sleep(0.3)]); settle()
    xdo(sum([['mousedown 1', sleep(GAP / 2), 'mouseup 1', sleep(GAP / 2)] for _ in range(100)], [])); sent += 200; settle()
def S_toggle(): clicks(R['toggle'])
def S_checkbox(): clicks(R['checkbox'])
def S_idle():
    # first frame done (rects published after it) -> 1 s settle -> 5 s window, no input at all (METHOD §6)
    time.sleep(1.0)
    c0, v0 = cpu_ticks(), vol_ctxt(); w0 = time.time_ns(); time.sleep(5.0); w1 = time.time_ns(); c1, v1 = cpu_ticks(), vol_ctxt()
    idle.update(window_start_ns=w0, window_end_ns=w1, cpu_ticks=c1 - c0, vol_ctxt=v1 - v0, gl_calls_in_window=None, idle_state='idle-initial')
    if a.specimen == 'F01':
        xdo([mv(R['name']), 'click 1', sleep(0.3)]); settle(0.5)
        r0 = nrows(); time.sleep(5.0); idle['focused_info_only'] = {'frames_in_5s': nrows() - r0}
SC = {'ptr-move': S_ptr_move, 'tab': S_tab, 'type-latin': S_type_latin, 'ime-zh': S_ime, 'toggle-show': S_toggle, 'checkbox': S_checkbox, 'idle': S_idle, 'first-frame': lambda: None}
ALL = {'L01': ['ptr-move'], 'F01': ['tab', 'type-latin', 'toggle-show', 'checkbox']}   # ime-zh joins `rss` only if requested (needs fcitx)
scen_run = [a.scenario]
if a.scenario == 'rss':
    scen_run = ALL[a.specimen] + (['ime-zh'] if os.environ.get('PVC_RSS_INCLUDE_IME') == '1' and a.specimen == 'F01' else [])
    for s in scen_run: SC[s]()
    time.sleep(0.5)
else:
    SC[a.scenario]()
settle(0.5)

# ---------- RSS: read while the host is alive (METHOD §8) ----------
rss = {}
for l in open(f'/proc/{pid}/status'):
    k = l.split(':')[0]
    if k in ('VmHWM', 'VmRSS', 'RssAnon', 'RssFile', 'RssShmem'): rss[k.lower() + '_kb'] = int(l.split()[1])
# non-PVC processes, for information only
def rss_of(name):
    tot = 0
    for d in os.listdir('/proc'):
        if d.isdigit():
            try:
                if open(f'/proc/{d}/comm').read().strip() == name:
                    for l in open(f'/proc/{d}/status'):
                        if l.startswith('VmRSS'): tot += int(l.split()[1])
            except Exception: pass
    return tot
rss['info_not_included'] = {'Xvfb_rss_kb': rss_of('Xvfb'), 'fcitx5_rss_kb': rss_of('fcitx5')}
probe_after = idle_probe(1.5)
my_cpu = sum(os.times()[:2])
proc.send_signal(signal.SIGTERM)
try: proc.wait(timeout=10)
except subprocess.TimeoutExpired: proc.kill()
host_log.close()
time.sleep(0.2)

meta = json.load(open(metaf)) if os.path.exists(metaf) else {}
# normalise the host trace to the §3 header (host may append extra columns such as n_events)
HDR = 'frame_idx,t_ns,scenario,event,update_ms,layout_ms,scene_ms,a11y_ms,mirror_ms,render_ms,total_ms,completion_ms'.split(',')
rows = list(csv.DictReader(open(trace)))
missing = [h for h in HDR if rows and h not in rows[0]]
if missing: sys.exit(f'host trace missing columns: {missing}')
with open(trace, 'w', newline='') as f:
    w = csv.writer(f); w.writerow(HDR)
    for r in rows: w.writerow([r[h] for h in HDR])
json.dump({
    'path': a.path, 'specimen': a.specimen, 'scenario': a.scenario, 'variant': None, 'mirror': None,
    'atspi': os.environ.get('PVC_ATSPI'), 'mode': 'windowed' if a.path == 'tier0c-vello_cpu' else 'windowed(gl,wgpu GL backend, downlevel_webgl2 limits)', 'rep': int(rep), 'cmd': a.cmd,
    'date_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), 'loadavg_start': loadavg, 'idle_probe_before_pct': probe, 'idle_probe_after_pct': probe_after, 'noisy': noisy,
    't_spawn_ns': t_spawn_ns, 'events_sent': sent, 'scenarios_run': scen_run,
    'artifacts_sha256': {'host_binary': sha(a.bin), 'signed_artifact': sha(a.artifact)},
    'toolchain': {'rustc': subprocess.run(['rustc', '-V'], capture_output=True, text=True).stdout.strip()},
    'host_meta': meta, 'idle': idle or None, 'idle_state': idle.get('idle_state'), 'rss': rss, 'harness_cpu_s': round(my_cpu, 2), 'frames_recorded': len(rows),
}, open(os.path.join(a.out, f'runmeta_rep{rep}.json'), 'w'), indent=1)
print(f'rep {rep} {a.specimen}/{a.scenario}/{a.path}: frames={len(rows)} events_sent={sent} noisy={noisy}')
