# REPORT — dev-native (A4: native host `pvc-native`)

Written at session wrap-up, state as-is. All paths are under `/workspace/dev_work/dev-native/` unless absolute.
Host for everything below: `host=container-xeon4` (Linux/X11 under Xvfb, Mesa 25.2.8 llvmpipe). **No number here is a Tier 1 device result; nothing ran on a phone or Chromebook (PLAN V1–V4).**

## 1. Summary
- `pvc-native` (winit 0.30.13 X11 window 1366×768 → wgpu 30.0.1 GL/llvmpipe with WebGL2 downlevel limits → vello_gpu via `pvc_runtime::render_gl::GlRenderer`; `--renderer cpu` = vello_cpu windowed) loads a signed artifact through `pvc_ir::load_signed`, runs the real `pvc_runtime` on F01 and L01, and presents only when `needs_frame()` is true.
- **System IME works, not synthetic:** fcitx5 5.1.7 + pinyin over XIM under Xvfb delivered `Ime::Preedit("ni hao")` then `Ime::Commit("你好")` (candidate 1) to winit; the runtime drew the preedit in F01's name field, the commit landed in runtime state (`name=Text("你好")`) and on screen.
- **AT-SPI works:** an external `pyatspi` client reads the live accesskit_winit tree: F01 roles/names correct, checkbox `checked` change after a key press (dump + event), the error message arrives as an `object:announcement` (AT-SPI carrier of `live=polite`).
- **Partly met:** the committed `你好` is **not** visible in the AT-SPI dump, because `pvc_runtime` exposes no Text/Value interface for text inputs (no TextRun child; dev-render's known gap, reported to them). Verified in runtime state and screenshot only.
- Idle: 1 frame (init) and 0 more in a 5 s window, both renderers (host-level witness only). Tamper refusal tested (4 tests pass). Hard-wall check clean (one flagged item: zbus/async-io via accesskit_unix).
- Hover: the runtime has no hover state, so `ptr-move` on L01 yields 0 frames (ruled: labelled "no hover affordance by design").
- **Handoff item #1 (Cargo.toml pvc-ir path): see §7.**

## 2. Done criteria (brief items 1–5 and "Done means")
| # | Criterion | Status | Evidence |
|---|---|---|---|
| 1a | winit 0.30.13 window 1366×768 on X11/Xvfb, own display `:77` | **met** | `scripts/env_up.sh`; `evidence/run_out/*/shot` PNGs (1366×768) |
| 1b | wgpu 30.0.1 GL surface on llvmpipe, vello_gpu rendering via dev-render's GL path | **met** | `evidence/run_out/a11y/meta.json` (`backend":"Gl"`, llvmpipe, `limits":"downlevel_webgl2_defaults"`, Rgba/Bgra8Unorm surface) |
| 1c | `ControlFlow::Wait`, redraw only when `needs_frame()`, frames counted | **met** | `src/app.rs`; idle: `evidence/sanity_shared_lock/{gl,cpu}/idle/idle.txt` (1→1 rows in 5 s); `frames_presented` in every `meta.json` |
| 2 | CLI `pvc-native <signed-artifact> [--locale]`, verify-on-load, refuses tampered artifact, test with tampered fixture | **met** | `pvc-native/tests/tamper.rs`; `logs/tamper_test.log` (4 passed). Dev-ir has no tampered *file* fixture (only an in-test builder), so my test edits copies of `F01.pvca`/`L01.pvca` (manifest field edit without re-sign; flipped byte); exit code 3 with DISPLAY unset proves the refusal precedes any windowing |
| 3a | accesskit_winit 0.34.1 fed with `pvc_runtime` TreeUpdate; actions handled | **met for Click/Focus path; SetValue/ReplaceSelectedText partly** | `src/app.rs` (`ActionRequested` → `rt.a11y_action`). **Not exercised by any external client** (no AT-SPI action was sent in my runs; `a11y_actions: 0`) → *inferred*, not checked |
| 3b | `at-spi2-core python3-pyatspi`, dbus + registry, `atspi_dump.py` | **met** | `scripts/atspi_dump.py`, `scripts/env_up.sh`; apt versions §6 |
| 3c | Dump shows every F01 interactive element with correct role and name | **met** | `evidence/run_out/a11y/a11y_0_initial.txt` (table below) |
| 3d | Checkbox state change driven by key press visible in second dump | **met** | `a11y_1_after_checkbox_space.txt` (`check box … checkable,checked,focused`), event `object:state-changed:checked` in `atspi_events.jsonl`, runtime state `terms=Bool(true)` in `a11y_1_checkbox.state.json` |
| 3e | F01 error message exposed live/polite or AT-SPI equivalent | **met** | `evidence/run_out/a11y/atspi_events.jsonl`: `object:announcement`, `detail1=1`, text "Enter a valid email address, like name@example.com", source `status bar`; the node is in `a11y_2_invalid_email.txt`. (AT-SPI has no static "live" attribute here: accesskit_atspi_common emits an Announcement when a live node's name changes — `accesskit_atspi_common-0.21.0/src/node.rs:631`, `adapter.rs:72`. `detail1=1`↔Polite is *inferred* from that source mapping `Live::Polite→Politeness::Polite`.) |
| 4a | fcitx5+chinese-addons+xdotool installed; XIM configured; `set_ime_allowed` on text-field focus; IME cursor area from runtime | **met** | `config/`, `scripts/env_up.sh`; meta `ime_set_allowed_calls:3`, `ime_set_cursor_area_calls:7` (`evidence/run_out/a11y/meta.json` / `ime_zh/meta.json`) |
| 4b | xdotool `nihao` + first candidate → winit `Ime::Preedit` then `Ime::Commit("你好")`; record what the candidate was | **met** | `evidence/run_out/ime_zh/host.log` and `meta.json` `ime_events`: `Preedit("n")`, `("ni")`, `("ni h")`, `("ni ha")`, `("ni hao")`, `Preedit("")`, `Commit("你好")`. Candidate list on screen: 1.你好 2.你 3.👋 4.拟 5.逆 6.泥 7.腻 (`ime_1_preedit.png`). Earlier bare-winit probe (no pvc code): `logs/EVIDENCE_ime_probe_system_fcitx5.log`, `pvc-native/examples/ime_probe.rs` |
| 4c | preedit drawn in F01's name field; committed text in runtime state | **met** | `ime_1_preedit.png` (preedit "ni hao" drawn in the field, partly under fcitx's candidate window), `ime_2_commit.png` (你好 in field), `ime_2_commit.state.json` (`name=Text("你好")`) |
| 4d | committed text in the AT-SPI dump | **NOT met** | `ime_2_commit.txt/.json`: `entry 'Full name'` has interfaces `Accessible, Component` only; no text. Cause: no TextRun child / text selection in the runtime TreeUpdate (dev-render's labelled gap; recipe sent to them). My earlier *placeholder* runtime (not pvc_runtime; `evidence/placeholder_runtime_phase/ime_zh/ime_2_commit.txt`) exposed `text='你好'` through the same host path, which shows the host/accesskit_winit path is not the limit — labelled **placeholder, not PVC** |
| 5a | `PVC_TRACE` CSV in METHOD schema, frames counter, first-frame, VmHWM | **met** | `CHECKPOINT.md`; `evidence/sanity_shared_lock/*/trace.csv`, `meta.json` (adds one extra column `n_events` after `completion_ms`, allowed by METHOD §3) |
| 5b | scripted-input mode for dev-measure | **met (scripts exist and ran)** | `scripts/scenarios/{ptr-move,tab,type-latin,toggle-show,checkbox,idle,ime_zh,a11y}.sh`, `scripts/run_native.sh`; `scripts/sanity_all.sh` ran them all |
| – | `run_native.sh` end to end (Xvfb+dbus+AT-SPI+IME+scenario) | **met** | `scripts/run_native.sh`; outputs in `evidence/run_out/`, `evidence/sanity_shared_lock/` |
| – | `cargo tree -e normal` saved, no HTTP/LLM/net crate | **met** (with a flag) | `logs/cargo_tree_native_normal.txt`, `logs/hardwall_check.txt` |
| – | REPORT with non-Tier-1 labelling | this file | |

F01 AT-SPI dump (`evidence/run_out/a11y/a11y_0_initial.txt`), interactive elements only:
| F01 element | AT-SPI role | accessible name | notes |
|---|---|---|---|
| name field | `entry` | "Full name" | editable, required, single_line |
| email field | `entry` | "Email" | |
| password field | `password text` | "Password" | |
| show/hide toggle | `toggle button` | "Show password" | |
| terms checkbox | `check box` | "I accept the terms of service" | `checked` after Space |
| submit | `push button` | "Sign up" | disabled (no `enabled/sensitive`) until valid |
Not interactive but present: form "Create account", heading, 4 labels/help `label`s, 3 `status bar` live nodes (empty names while hidden). Root `frame` has an empty name (runtime sets no window label): minor, flagged.

## 3. Checked vs inferred
**Checked (I ran it; file given):**
- System IME → winit → runtime → screen chain (§2 4a–4c). `evidence/run_out/ime_zh/`, `logs/EVIDENCE_ime_probe_system_fcitx5.log`.
- Needed fcitx5 config: XIM addon option `UseOnTheSpot=True` (`config/conf/xim.conf`); without it only `Commit("你好")` arrived and **no `Preedit` text** (first probe, shown in my session, log overwritten; the working log is the evidence above). Pinyin must be activated with Ctrl+Space after the profile default `keyboard-us`; with pinyin active, Latin letters go to the IME (observed: typing "bad" produced `Preedit("ba d")` instead of text).
- AT-SPI tree, checkbox state, error announcement (§2). `evidence/run_out/a11y/`.
- Tamper refusal tests (`logs/tamper_test.log`), hard-wall cargo tree (`logs/hardwall_check.txt`), G7 versions in `pvc-native/Cargo.lock`: wgpu 30.0.1, vello_cpu 0.3.0, vello_gpu 0.3.0, parley 0.11.1, fontique 0.11.1, harfrust 0.12.0, skrifa 0.44.0, accesskit 0.25.1, accesskit_winit 0.34.1, winit 0.30.13, glifo 0.4.0 (one version each; parley/fontique/harfrust/skrifa/glifo arrive via pvc-runtime).
- Idle: no frame after init in 5 s on both renderers (`evidence/sanity_shared_lock/{gl,cpu}/idle/idle.txt`). Witness is the host's own trace rows only; the CPU-tick witness of METHOD §6 was not taken.
- Hover: L01 `ptr-move`: `events_received=404`, `frames_presented=1` (`evidence/sanity_shared_lock/gl/ptr-move-L01/meta.json`). Cause read from `pvc-runtime/src/runtime.rs` `pointer_move` (drag-select only).
- winit 0.30.13 defect W1 (§7): host aborted at exit with a composition in progress (panic text `failed to send preedit update event: SendError`, from the host.log of a placeholder-runtime a11y run; that log was overwritten by the next run and is **not preserved** — treat as *observed once, log not preserved*; the mitigation in `src/app.rs` `UserEvent::Term` ran cleanly in all later runs: `host_exit=0` in every `run.txt`).

**Inferred / not checked:**
- AccessKit `SetValue`/`ReplaceSelectedText`/`Click` actions from an external AT: code path written, never exercised (`a11y_actions:0`).
- `detail1=1` = polite (from accesskit_atspi_common source, not from the AT-SPI spec text).
- `--locale zh-CN` F01 rendering: **not run** (only `en` was exercised); runtime strings table is dev-ir's.
- Typing 20 Latin chars into the name field producing the right text: `type-latin` ran (112 frames, 117 events) but I did not inspect the resulting text; the scenario clears the field at the end of each round.
- L01 rendering on the native host: ran (`ptr-move-L01`, 1 frame) but I did not look at a screenshot.
- Anything about Crostini/ChromeVox/Android IME: **unverified and stays unverified**.
- Whether fcitx5 under XIM reflects a real desktop IME: it is a real IME engine over a real XIM protocol, but with no desktop session, no WM, `xdotool windowfocus` standing in for window-manager focus.

## 4. Measurements (all host-labelled; none are the official A6 measurement — dev-measure owns those)
`evidence/sanity_summary.txt` (generated from the raw CSVs listed there). `host=container-xeon4`, release build, **shared** CPU lock only (exclusive lock starved behind other Developers' jobs; loadavg at each start is in each `run.txt`, 0.46–1.06), `path=gl-llvmpipe` (gl/) and `path=tier0c-vello_cpu mode=windowed, present = texture upload + GL blit` (cpu/). General constraint solve excluded (D-4). NOT a Tier 1 device result.
| cell (F01 unless noted) | p50 / p95 / max completion_ms (first 10 dropped) | VmHWM kB | first frame ms from `main()` |
|---|---|---|---|
| gl checkbox / toggle-show / tab / type-latin | 6.95/9.48/10.81 · 7.20/11.91/17.31 · 6.86/13.41/16.46 · 6.08/11.93/13.83 | 139264–140208 | 108–149 |
| cpu checkbox / toggle-show / tab / type-latin | 10.00/13.26/20.20 · 10.49/15.96/25.01 · 11.13/21.67/23.08 · 8.11/10.58/12.59 | 137232–137716 | 87–104 |
These carry n≈51–101 frames each, one process, **no repetition statistics, no exclusive lock**: they show only that the hooks produce plausible rows. Do not quote them as budget results. VmHWM 137–140 MB is under 153 600 kB but includes llvmpipe/LLVM libraries and the 16 MB CJK font; no baseline subtraction.
First-frame is `main()` entry → first frame completed, not exec → frame.

## 5. Deviations and labelled gaps
- No Tier 1 device; Linux/X11/Xvfb, no WM, no compositor, no vsync; llvmpipe is CPU (V1–V4).
- `--renderer cpu` is windowed through the wgpu GL surface (GL surface offers only RENDER_ATTACHMENT, so the vello_cpu pixmap is uploaded to a texture and drawn by a 3-vertex WGSL blit): label `mode=windowed, present=upload+GL blit`, not an X11 PutImage path. Added a direct `vello_cpu =0.3.0` dep (pinned, in G7 table) to the host for it.
- wgpu/winit/accesskit_winit features narrowed (wgpu: `std,wgsl,gles`; winit: `x11,rwh_06`; accesskit_winit: `accesskit_unix,async-io,rwh_06`) — same versions, fewer backends; Wayland/Vulkan not compiled.
- IME candidate window (fcitx5 classicui) is a separate X window overlapping the field; the screenshot shows the preedit partly under it. `set_ime_cursor_area` was driven by `rt.focused_text_input_rect()`; the window opens just below that rect.
- Masked fields: runtime ignores preedit (dev-render gap) — not tried by me.
- No clipboard, no caret blink (runtime), no hover (ruled).
- SIGTERM handling uses a direct `extern "C" signal` declaration (no crate), a watcher thread polls the flag every 25 ms and wakes the loop with a user event (only in the harness path; costs one sleeping thread, no frames).
- Trusted key = dev-ir TEST key (public seed). Not a deployment key policy.

## 6. Dependencies
**Cargo (pvc-native):** all in the G7 table, exact `=` pins as in the brief, plus path deps `pvc-runtime` (features `gl`,`cpu`) and `pvc-ir`. No crate outside the table was added → **no RFC notes written** (none needed). `Cargo.lock` sha256 `c8040e71ee05016d…` (see §7). Transitive: `accesskit_unix 0.24.0` pulls `zbus 5.19.0`, `async-io 2.6.0`, `polling 3.11.0`.
**apt (installed by me, D-9), versions from `dpkg -l`:** at-spi2-core 2.52.0-1build1, python3-pyatspi 2.46.1-1, fcitx5 5.1.7-1build3, fcitx5-chinese-addons 5.1.3-1build3, fcitx5-frontend-gtk3 5.1.1-1build2, xdotool 1:3.20160805.1-5build1, x11-apps 7.7+11build3, x11-utils 7.7+6build2, x11-xserver-utils 7.7+10build2, imagemagick 8:6.9.12.98+dfsg1-5.2build2, dbus-x11 1.14.10-4ubuntu4.1, libxkbcommon-x11-0 (as pulled; version not recorded). Not on the brief's list but installed as helpers: fcitx5-frontend-gtk3, x11-apps/utils/xserver-utils (xwd, xdpyinfo), imagemagick (xwd→png), dbus-x11 (dbus-launch). `ibus` fallback not installed (not needed). Preexisting: Xvfb 2:21.1.12-1ubuntu1.6. Install log: `logs/apt_install.log`.
**Tooling:** system `python3.12` (`/usr/bin/python3.12`) for pyatspi; python3 (3.11, `/usr/local/bin`) only for JSON helpers in shell scripts.

## 7. Inconsistencies flagged / handoff items
1. **HANDOFF #1 — `pvc-ir` path in `pvc-native/Cargo.toml`.** The Supervisor's wrap-up note says it still points at dev-render's scaffold snapshot. **State as I leave it (verify with `grep pvc-ir pvc-native/Cargo.toml`):** I changed it to `/workspace/dev_work/dev-ir/pvc-ir` when dev-render's `pvc-runtime/Cargo.toml` switched to `../../dev-ir/pvc-ir` (a second path made a lockfile collision), then rebuilt, ran the tamper tests and built release. **But:** every *evidence run* in `evidence/` (`run_out`, `sanity_shared_lock`) was produced by binaries built while the path was `/workspace/dev_work/dev-render/scaffold/pvc-ir-a1` (the A1 snapshot), and the binary shas at run time were **not recorded** and those binaries have since been overwritten in the shared target dir. Current binaries (post-switch, never used for an evidence run): `/workspace/cargo-target/debug/pvc-native` sha256 `4c3a4f80dae351900f80e7ee8005f9d4245dd4786e03c2fb33656ab437a7b111`, `/workspace/cargo-target/release/pvc-native` sha256 `29d3b94106e2165073596741abf47f88ab38e4118ccd714efccd156330a197d6`; `Cargo.toml` sha256 `b31713f8f627f9bfa64b9a9150650d69095427aea842ddf082cd7710feabc354`. Integration must re-run `scripts/run_native.sh a11y|ime_zh` against the final pvc-ir/pvc-runtime to re-confirm; artifact canonical sha256 in the evidence runs: `81b12fdb25ac447f37dc96da8b2820eb5e81c702dbe5278a8af285b27e41cbd7` (F01).
2. **Disk/target dir:** `pvc-native/target` was deleted; builds go to `/workspace/cargo-target`. Path in `run_native.sh` defaults to that.
3. **`/usr/bin/python3` is 3.11 here; apt `pyatspi`/`gi` is built for 3.12.** The brief's "system `/usr/bin/python3`" does not import pyatspi; scripts use `/usr/bin/python3.12`.
4. **METHOD `ptr-move` presumes hover frames; runtime has none** (ruled: cell labelled "0 frames — no hover affordance by design; idle rule holds").
5. **AT-SPI has no text for inputs** (dev-render gap, §2 4d) and root window label is empty.
6. **winit 0.30.13 W1:** `EventProcessor::drop` can panic (`failed to send preedit update event`, `x11/ime/context.rs:123`) → abort ("panic in a function that cannot unwind") if the process exits while a pinyin composition is active. Mitigated in the host (write meta, `set_ime_allowed(false)`, drop window before `exit`). Upstream bug, observed once (log not preserved).
7. **Hard-wall note:** `accesskit_unix` brings `zbus`/`async-io`/`polling` (D-Bus over a Unix socket, for AT-SPI). They are not HTTP/LLM clients and no crate in the reqwest/hyper/ureq/curl/openai/tokio/rustls family is in `cargo tree -e normal`; but `async-io` contains TCP/UDP socket types. Supervisor's grep on crate names is clean; a strict "no socket-capable crate" reading would fail on the native a11y path.
8. README/docs: brief says "`/usr/bin/python3`, which has pyatspi" — not true here (item 3).

## 8. Open issues
- 4d (committed text in AT-SPI) needs dev-render's TextRun + selection; after that re-run `scripts/run_native.sh ime_zh` and check `ime_2_commit.txt`.
- Re-run evidence with final binaries (handoff #1); record the binary sha in each `run.txt` (not done by `run_native.sh` yet).
- `--locale zh-CN` native run and F01 Latin-typing text check not done; zh-CN IME with locale zh-CN not run (IME run used `--locale en`).
- Exclusive-lock measurements, repetitions, CPU-tick idle witness, AT-SPI-attached frame-cost variant (`atspi=attached`: the a11y run had a client attached but I did not extract per-frame a11y_ms from it) — dev-measure's.
- No test of AT-SPI-initiated actions (Click/SetValue) from an external client.
- Packaging: `evidence/` and `run/` contain generated logs only; `archive/` holds the placeholder runtime (`placeholder.rs`, `ui.rs`) used before pvc_runtime existed — **never a PVC result**; its outputs are in `evidence/placeholder_runtime_phase/` and each is labelled by that directory name.

## 9. File index
`CHECKPOINT.md` (for dev-measure), `pvc-native/` (crate: `src/{main,app,gpu,loader,trace}.rs`, `tests/tamper.rs`, `examples/ime_probe.rs`), `scripts/` (`env_up.sh`, `run_native.sh`, `sanity_all.sh`, `atspi_dump.py`, `atspi_listen.py`, `lib.sh`, `scenarios/`), `config/` (fcitx5 profile, `conf/xim.conf`), `logs/`, `evidence/`, `archive/`.
