# CHECKPOINT — dev-native (pvc_native host)

Crate: `/workspace/dev_work/dev-native/pvc-native/` (bin `pvc-native`, lib `pvc_native`). Build with the shared target dir:
`CARGO_TARGET_DIR=/workspace/cargo-target cargo build --release` in the crate dir → `/workspace/cargo-target/release/pvc-native`.
(Release profile: opt-level 3, lto thin, codegen-units 4, debug off — from the crate's `Cargo.toml`.)

## 2026-10-08 — hooks work; for dev-measure (METHOD.md v1 §11)

### Launch
```
pvc-native <signed-artifact.pvca> [--locale en|zh-CN] [--renderer gl|cpu]
```
* artifacts: `/workspace/dev_work/dev-ir/specimens/{L01,F01}.pvca` (signed with dev-ir's TEST key; any tampered/other-signer artifact → exit code 3, message `refusing artifact`, before any window exists).
* `--renderer gl` (default) = **path `gl-llvmpipe`**: winit 0.30.13 X11 window 1366×768 + wgpu 30.0.1 GL backend (Mesa llvmpipe) + vello_gpu via `pvc_runtime::render_gl::GlRenderer`, device created with `Limits::downlevel_webgl2_defaults()` (meta.json `"limits"` says so).
* `--renderer cpu` = **path `tier0c-vello_cpu`, `mode=windowed`**: vello_cpu rasterises the same display list into a pixmap each frame (persistent context), pixels are uploaded to a texture and blitted to the **same wgpu-GL surface**. So its present step is a texture upload + GL draw + present on llvmpipe, *not* an X11 PutImage. Label it as such (meta.json `renderer_mode`).
* `--locale zh-CN` works for F01 (L01 declares only `en`; the runtime refuses `zh-CN` for L01).
* Window: exactly 1366×768 physical at scale 1.0 (no WM under Xvfb; at (0,0), so screen coords = window coords). Needs `xdotool windowfocus <id>` for keyboard focus (no WM).

### Env (all optional; unset = zero overhead)
| env | effect |
|---|---|
| `PVC_TRACE=<path>` | per-frame CSV, header = METHOD §3 **plus one extra last column `n_events`** (events coalesced in the frame). One row per presented frame, written after `completion_ms` is sampled. `mirror_ms` is empty. |
| `PVC_SCENARIO=<id>` | copied verbatim into `scenario` |
| `PVC_META=<path>` | meta.json written at SIGTERM/SIGINT (and window close). Keys: `label, renderer_mode, frames_presented, events_received, renderer, backend, vmhwm_kb, vmrss_kb, rssanon_kb, rssfile_kb, rssshmem_kb, build{git_or_sha256 = artifact canonical sha256, profile}, limits, surface_format, present_mode, first_frame_ms_from_main_entry, locale, artifact, a11y_activated, a11y_updates_sent, a11y_actions, ime_set_allowed_calls, ime_set_cursor_area_calls, ime_events[], …`. **VmHWM is read inside the host at SIGTERM**; the process stays alive until SIGTERM, so the harness may also read /proc/<pid>/status itself. |
| `PVC_RECTS=<path>` | JSON written after the first frame: keys `name,email,password,toggle,checkbox,submit` (F01), `connect_button,bio_text` (L01) + every visible IR id → `[x,y,w,h]` logical px (= screen px here). |
| `PVC_STATE=<path>` | `rt.debug_state()` rewritten after every frame (**costs file I/O per frame: do not set it during timed runs**). |

### Timing definitions as implemented (METHOD §2.2)
* `t_ns` = CLOCK_REALTIME ns at event receipt of the **earliest** event not yet followed by a frame. Row 0 (`init`): `t_ns` = process `main()` entry, so `t_ns + completion_ms` = first frame completion measured from main() entry (not from exec; the harness' `t_spawn_ns` is the exec-time figure).
* `update_ms` = host handler time (event→`rt.pointer_*/key/text/ime_*` returned, summed over coalesced events) + runtime update stage. `layout_ms/scene_ms` = runtime stages. `a11y_ms` = runtime a11y build + `accesskit_winit::update_if_active` hand-off (the closure — and so the AT-SPI D-Bus emission — only runs if an AT-SPI client activated the adapter; `a11y_activated` in meta says whether that happened). `render_ms` = surface acquire + draw + submit + `present()`. `total_ms` = event receipt → `present()` returned. `completion_ms` = same origin → after `device.poll(wait_indefinitely)` issued after total was sampled.
* No frame is presented for an input that leaves `needs_frame()` false (pending timestamp dropped). Idle = `ControlFlow::Wait`, no timers.
* `event` classes emitted: `init, ptr_move, ptr_down, ptr_up, key_tab, key_char, key_other, ime_preedit, ime_commit, resize, other (AT-SPI action), idle_redraw`. winit `Ime::Enabled/Disabled` events also count as `ime_preedit` class rows if they cause damage (rare).

### Scripted input (xdotool, real X11 events) — `/workspace/dev_work/dev-native/scripts/`
`run_native.sh <scenario> [--artifact F] [--locale L] [--renderer gl|cpu] [--bin P] [--out DIR] [--keep-env]` brings up Xvfb `:77` (1366×768×24), a private dbus session, at-spi-bus-launcher + registryd, fcitx5 (pinyin, XIM, on-the-spot) → runs the host with PVC_TRACE/META/RECTS/STATE set → runs `scenarios/<scenario>.sh` → SIGTERM → `--out` (default `run/out/<scenario>`).
Scenarios: `ptr-move` (env `PTR_IN=connect_button PTR_OUT=bio_text N=200`, 2 moves per iteration, 50 ms gap), `tab` (`N=60`), `type-latin` (`ROUNDS=5`), `toggle-show`, `checkbox` (`N=100` clicks), `idle`, `ime_zh` (system IME), `a11y` (AT-SPI dumps + event log). For timed runs you can keep using `scripts/env_up.sh` + your own driver: `source scripts/env_up.sh` exports DISPLAY=:77, DBUS_SESSION_BUS_ADDRESS, XMODIFIERS=@im=fcitx. **Latin typing needs the fcitx5 default layout `keyboard-us` (profile default); the pinyin IM is switched on only by `ime_zh` with Ctrl+Space** — typing letters while pinyin is active feeds the IME, not the field.
Setting `PVC_IM=none` before sourcing env_up.sh starts no IME daemon (then winit gets no XIM).

### AT-SPI tools
`scripts/atspi_dump.py [--brief] [--out F]` (**must run with `/usr/bin/python3.12`**: `/usr/bin/python3` is 3.11 here and cannot import the apt `pyatspi`). `scripts/atspi_listen.py OUT SECS` logs `object:announcement` (accesskit_atspi_common's live-region carrier) and state/name/text events.

### Known limits (labelled)
* Linux/X11 under Xvfb + llvmpipe; not Android/ChromeOS; no vsync, no compositor, no window manager.
* `--renderer cpu` present path described above.
* Host RSS includes llvmpipe/LLVM and the 16 MB Noto CJK font (runtime bundle).
