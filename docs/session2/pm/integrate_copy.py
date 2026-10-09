#!/usr/bin/env python3
"""Checkpoint integration copy (Orchestrator). Copies dev_work evidence into the repo, excluding build outputs."""
import shutil, os, pathlib
R = pathlib.Path('/workspace/Project-VectorCanvas'); D = pathlib.Path('/workspace/dev_work'); P = pathlib.Path('/workspace/orchestrator_work/pm')
EXC = {'target', 'node_modules', '.venv', '__pycache__', 'pkg', 'work', 'xdg-cache', 'xdg-config', 'xdg-data', 'xdg-runtime', 'home'}
def ign(dirpath, names):
    return [n for n in names if n in EXC]
def cp(src, dst):
    src = pathlib.Path(src); dst = pathlib.Path(dst)
    if not src.exists():
        print('MISSING', src); return
    dst.parent.mkdir(parents=True, exist_ok=True)
    if src.is_dir():
        shutil.copytree(src, dst, ignore=ign, dirs_exist_ok=True)
    else:
        shutil.copy2(src, dst)
# code
cp(D/'dev-ir/pvc-ir', R/'crates/pvc-ir')
cp(D/'dev-render/pvc-runtime', R/'crates/pvc-runtime')
cp(D/'dev-native/pvc-native', R/'hosts/pvc-native')
cp(D/'dev-native/scripts', R/'hosts/pvc-native-scripts')
cp(D/'dev-native/config', R/'hosts/pvc-native-scripts/config')
for n in ['pvc-web', 'www', 'tests', 'build.sh', 'run_web.sh', 'cargo_tree_wasm32.txt']:
    cp(D/'dev-web'/n, R/'hosts/pvc-web-host'/n)
cp(D/'dev-ir/specimens', R/'specimens')
cp(D/'dev-measure/tools', R/'tools')
# evidence
M = R/'measurements/session2'
cp(D/'dev-render/measurements/golden', M/'golden')
for s in ['d3', 'hardwall', 'pins', 'rss', 'frame_bench']:
    cp(D/'dev-render/measurements'/s, M/f'dev-render-{s}')
cp(D/'dev-render/measurements/tests.log', M/'dev-render-tests.log')
# text-bar: keep work/ PNGs too (raw evidence) -> copy without the EXC filter for 'work'
shutil.copytree(D/'dev-measure/measurements/session2/text-bar', M/'text-bar', dirs_exist_ok=True)
for n in ['out', 'sanity']:
    cp(D/'dev-native/run'/n, M/'native-as-left'/n)
cp(D/'dev-native/logs', M/'native-as-left/logs')
for f in ['dump1.json', 'dump2.json', 'meta.json', 'shot1.png', 't.csv', 'rects.json', 'state.json']:
    cp(D/'dev-native/run'/f, M/'native-as-left/early-smoke'/f)
cp(D/'dev-web/out', M/'web-as-left/out')
# dev docs
for n in ['dev-ir', 'dev-render', 'dev-native', 'dev-web', 'dev-measure', 'dev-pilot', 'dev-baseline']:
    for f in ['REPORT.md', 'CHECKPOINT.md', 'GRAMMAR.md', 'PILOT_COVERAGE.md', 'METHOD.md', 'TOLERANCE.md', 'PVC_CHECKER_NEEDS.md', 'README.md', 'TOOLS.md']:
        if (D/n/f).exists(): cp(D/n/f, R/'docs/session2/dev'/n/f)
    if (D/n/'rfc').exists(): cp(D/n/'rfc', R/'docs/session2/dev'/n/'rfc')
cp(D/'dev-ir/evidence', R/'docs/session2/dev/dev-ir/evidence')
cp(D/'dev-ir/d1-wasm', R/'docs/session2/dev/dev-ir/d1-wasm')
for n in ['d3-wasm', 'golden-web', 'tools']:
    cp(D/'dev-render'/n, R/'docs/session2/dev/dev-render'/n)
cp(D/'dev-render/bin/SHA256SUMS', R/'docs/session2/dev/dev-render/bin_SHA256SUMS')
# pilot
cp(D/'dev-pilot/harness', R/'pilot/harness')
cp(D/'dev-pilot/intents', R/'pilot/intents')
cp(D/'dev-pilot/dryrun', R/'pilot/DRY-RUN-not-data/dryrun')
cp(D/'dev-pilot/dryrun_aborted_F01_starved', R/'pilot/DRY-RUN-not-data/dryrun_aborted_F01_starved')
cp(D/'dev-ir/pilot-refs', R/'pilot/pvc-refs-reference-only')
for n in ['baseline', 'pvc', 'examples']:
    cp(D/'dev-baseline'/n, R/'pilot/checker'/n)
# pm
for f in ['PLAN.md', 'STATUS.md', 'QUIET_WINDOW.md']:
    cp(P/f, R/'docs/session2/pm'/f)
cp(P/'briefs', R/'docs/session2/pm/briefs')
F = R/'docs/session2/pm/feasibility'
for f in ['FINDINGS.md', 'webgl2_probe.mjs', 'webgl2_probe.out', 'wgpu_adapters.out', 'host_cpu.out', 'pinprobe.Cargo.lock']:
    cp(P/'feasibility'/f, F/f)
cp(P/'feasibility/pinprobe/Cargo.toml', F/'pinprobe.Cargo.toml')
cp(P/'feasibility/pinprobe/src/main.rs', F/'pinprobe.main.rs')
cp(P/'feasibility/wasmprobe/Cargo.toml', F/'wasmprobe.Cargo.toml')
print('done')
