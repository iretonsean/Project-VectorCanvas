# METHOD.md — PVC Session 2 measurement method (A6, step 1)

**Author:** dev-measure. **Version:** v1. **Status:** published *before any measurement exists*. No number in this session was produced before this file was written (check file mtimes: this file is the earliest file in `/workspace/dev_work/dev-measure/`).
**Audience:** the hosts (`pvc-native`, `pvc-web`) and `pvc-runtime` implement the schema in §3 and the hooks in §11. The Supervisor can re-run any single number with `tools/measure/run.sh` (§12, written in step 2).

Anything in this file marked **(decision)** is a choice I made before measuring; the docs do not specify it. Anything marked **(doc)** is quoted from the repo docs.

---

## 0. What is and is not being measured

**Host labels (COMMON §5). Every timing, RSS or FPS number carries exactly one of these, plus the mode flags in §1.3:**

| Path id | Label string used in every file |
|---|---|
| `tier0c-vello_cpu` | `host=container-xeon4 path=tier0c-vello_cpu` |
| `gl-llvmpipe` | `host=container-xeon4 path=gl-llvmpipe` (wgpu GL on Mesa 25.2.8 llvmpipe, WebGL2 downlevel limits imposed; the Tier 1 *code path* on a Tier 0 (c)-class machine) |
| `chromium141-webgl2-swiftshader` | `host=container-xeon4 path=chromium141-webgl2-swiftshader` (Chromium 141.0.7390.37 headless, `navigator.gpu` hidden, WebGL2 via ANGLE→SwiftShader, i.e. CPU) |

`container-xeon4` = 4 vCPU Intel Xeon @ 2.10 GHz, 15 GB, Ubuntu 24.04.5, no GPU.

**Required labels in every `summary.json` / `summary.md` (checked by `tools/measure/validate_labels.py` in step 2):**
1. the host label above;
2. the line `general constraint solve excluded (D-4): layout is the direct sugar solver`;
3. the line `NOT a Tier 1 device result; no phone or Chromebook was measured`;
4. sha256 of every artifact used: the host binary (native) or wasm + JS glue (web), the signed `.pvc` artifact, the font files;
5. `cat /proc/loadavg` at start of each repetition and the CPU-idle probe of §9;
6. the exact command line (`run.sh …` and the host command it expanded to);
7. UTC date, `rustc -V`, `cargo -V`, the cargo profile, the Chromium version string (web);
8. mode flags: `mode=windowed|offscreen` (tier0c), `variant=raf|sync` (web), `mirror=on|off` (web).

**Not measured / cannot be inferred from these numbers:** any phone, Chromebook, real GPU, real compositor, real vsync, a real IME engine on ChromeOS/Android, a general Cassowary solve, a screen reader.

## 1. Fixed test conditions

### 1.1 Specimens and viewport
- Specimens: **L01** (profile card) and **F01** (sign-up form), signed artifacts from `pvc-ir`, strings table `en` unless a scenario says `zh-CN`.
- Primary viewport **1366×768, device scale 1.0** (COMMON §7; §4.4.1 Tier 1 panel-native). Native: an X11 window of exactly 1366×768 on Xvfb `-screen 0 1366x768x24`. Web: Playwright `viewport={width:1366,height:768}`, `deviceScaleFactor=1`. No other viewport is measured for timing.
- Release builds only. The cargo profile (`opt-level`, `lto`, `codegen-units`, `debug`) is recorded from the host's `Cargo.toml` and written to the summary. A debug build is never measured.

### 1.2 Cold / warm definitions
- **Native first frame:** a fresh process; the binary and shared libraries are in the OS page cache (the harness does not and cannot drop caches). Xvfb, dbus and the IME daemon are already running and are not part of the figure. Labelled `warm-page-cache`.
- **Web first frame:** a fresh Chromium process and fresh browser context per repetition, HTTP cache disabled, served from a local static server on 127.0.0.1 (wasm, JS glue, font bytes and the artifact are fetched over loopback HTTP; that is inside the figure). The Chromium binary is warm in the page cache. Labelled `cold-context, warm-page-cache, loopback-served`.

### 1.3 Mode flags (must appear on every number)
- `tier0c-vello_cpu`: `mode=windowed` if the frame is presented to a window on Xvfb by a native host, else `mode=offscreen` (vello_cpu renders into a pixmap, the same `pvc_runtime` stages run, "present" is a no-op, events are synthesised in-process). The harness reports which one it measured; offscreen is acceptable per the brief **only if labelled**. Offscreen figures are a lower bound on a windowed figure.
- `chromium141-webgl2-swiftshader`: `variant=raf` or `variant=sync` (§4.2) and `mirror=on` (default, G4 §8.3) or `mirror=off` (control only, §4.3).

## 2. Interaction frame

An **interaction frame** is one frame presented by the host as the consequence of one input event (or one coalesced batch), measured from the point of **event receipt** to the point defined per path below. A frame that the host presents for no input event is an **idle-redraw** frame (§6), is tagged `event=idle_redraw`, and is never included in interaction statistics.

### 2.1 Scenarios (the six §4.4.1 rule 2 interactions + two non-interaction measures)

All scripts are driven **from outside the host** by the harness (native: `xdotool` against Xvfb, real X11 events; web: Playwright mouse/keyboard + CDP). Inter-event gap: **50 ms** after the previous event is sent (closed loop is not used; a fixed gap keeps the harness' own CPU use low and lets SwiftShader finish asynchronous work between frames). If a frame takes longer than the gap, events coalesce; this is detected by comparing events sent against frames recorded (§2.3) and is itself reported.

| Scenario id | Specimen | Script (exact) | Measured event class(es) | Frames measured per repetition |
|---|---|---|---|---|
| `ptr-move` | L01 | Pointer is put at the centre of the "Connect" button (setup, not measured). Then 200 pointer-move events alternating: out → in → out → …, where "out" is a point inside the card but on the bio text, "in" is the button centre. Every event therefore changes hover state, so every event *should* produce a frame. | `ptr_move` | ≤200 |
| `tab` | F01 | Focus starts on the document (click on empty background, not measured). 60 × `Tab`. The F01 tab order is name, email, password, show/hide, checkbox, submit (6 stops), so 10 full cycles. Disabled submit is skipped or focused as the host/runtime decides; that is recorded, not corrected. | `key_tab` | ≤60 |
| `type-latin` | F01 | Click in the name field (setup). 5 rounds of: type the 20 characters `Ada Lovelace Byron 1` (20 `key_char` events), then `Ctrl+A`, `Delete` (tagged `key_other`, not measured). | `key_char` | ≤100 |
| `ime-zh` | F01 | locale `zh-CN`, pinyin. Click in the name field (setup). 20 rounds of: the keystrokes `n`,`i`,`h`,`a`,`o` (5 preedit updates), then select candidate 1 (commit; the committed string is recorded, expected `你好`). | `ime_preedit`, `ime_commit` (reported separately and combined) | ≤120 |
| `toggle-show` | F01 | Pointer moved to the show/hide toggle (setup). 100 clicks (press+release). | `ptr_down`, `ptr_up` | ≤200 |
| `checkbox` | F01 | Pointer moved to the accept-terms checkbox (setup). 100 clicks. | `ptr_down`, `ptr_up` | ≤200 |
| `idle` | L01, F01 | §6. | — | — |
| `first-frame` | L01, F01 | §7. | — | — |
| `rss` | L01, F01 | §8. All of that specimen's interaction scenarios run in one process, RSS read at the end. | — | — |

Scenario × specimen cells not listed above are **not applicable** (L01 has no text field, so no `tab`/`type`/`ime`/`toggle`/`checkbox`; F01 `ptr-move` is not run because §4.4.1 names pointer move over the L01 button). They appear in SUMMARY.md as `n/a: <reason>`.

Native IME (`ime-zh`) is driven by `xdotool` through the system IME engine (fcitx5/ibus pinyin) set up by dev-native; web IME is driven by CDP `Input.imeSetComposition` + `Input.insertText` and is labelled **"browser IME composition pipeline driven via CDP; no OS IME engine involved"**. If dev-native could not get a system IME working under Xvfb, the native `ime-zh` cell is `not measurable here: <their reason>`; synthetic winit IME events are **never** reported as system IME (they may be measured and labelled `synthetic-ime`).

### 2.2 Where timing starts and stops

Notation: `t_recv` = event receipt; all durations in milliseconds as float64 from a monotonic clock.

**Native (`gl-llvmpipe`, windowed `tier0c-vello_cpu`):**
- `t_recv` = entry of the winit event callback that delivers the first input event not yet followed by a presented frame (for a batch, the earliest event). Clock: `std::time::Instant`.
- Start of stage clocks: `t_recv`.
- `update_ms`: `t_recv` → end of hit-test, runtime event handling, behaviour interpretation, text-editor edit / IME preedit application (runtime-reported).
- `layout_ms`: runtime layout pass (runtime-reported; 0 if layout was not dirty).
- `scene_ms`: scene build (shaping lookups, glyph runs, vello scene commands) (runtime-reported).
- `a11y_ms`: building the `accesskit::TreeUpdate` **and** handing it to `accesskit_winit` (`update_if_active`). Runtime reports the build, host adds the hand-off. When no AT-SPI client is connected, `accesskit_winit` skips work; the harness therefore records whether an AT-SPI client was attached (default: **not attached**; a variant with `pyatspi` attached may be run and labelled `atspi=attached`).
- `render_ms`: the renderer call (vello_cpu raster + blit to window buffer; or vello_gpu render + `queue.submit`) up to and including `present()` returning.
- `total_ms`: `t_recv` → `present()` returned. One continuous span, so any untracked glue appears as `total_ms − Σ stages`.
- `completion_ms`: `t_recv` → return of a forced GPU-completion wait issued **immediately after** `present()` (`device.poll(PollType::wait())` or the wgpu-30 equivalent; for `tier0c` raster is synchronous so `completion_ms = total_ms`). The wait is issued after `total_ms` has been sampled, so it cannot inflate `total_ms`. It is always on in measurement mode.
- `mirror_ms`: empty (native has no DOM mirror).

**Web (`chromium141-webgl2-swiftshader`):** the page's JS/wasm work for the frame, *not* the scheduler wait between the event and the next animation tick (headless rAF ticks at 60 Hz, so a latency-style figure would contain up to 16.7 ms of pure waiting; that would be a measurement artefact).
- `t_recv` = entry of the first DOM input event handler (pointer/key/composition/input) after the previous frame. Clock: `performance.now()` (Chromium clamps to 100 µs in non-isolated pages; resolution is stated in the summary).
- `update_ms`, `layout_ms`, `scene_ms`, `a11y_ms`: as native (runtime-reported; `update_ms` includes the DOM event handler work).
- `mirror_ms`: JS time to apply the `TreeUpdate` to the hidden DOM/ARIA mirror **plus one forced style/layout flush of the mirror subtree** (read of `mirrorRoot.getBoundingClientRect()` immediately after the mutations). **(decision)** Rationale: DOM mutation alone is cheap and the browser would otherwise defer style+layout to after the rAF callback, where `performance.now()` deltas would not see it; the flush work is work the browser must do anyway before paint, so forcing it moves it, it does not add it. This flush is part of `mirror_ms` in **both** variants. What is still not captured: compositor and AX-tree serialisation work in the browser process, done after the page's JS returns (labelled gap, §10).
- `render_ms`: from just after the mirror update to the return of the last GL-issuing call of the frame (the host's render call; includes vello_gpu CPU path preparation, uniform/texture uploads and the GL draw calls issued, **not** their execution on the GPU process).
- `total_ms`: Σ (durations of the event-handler work for the events coalesced into this frame) + (rAF callback entry → rAF callback exit), each from `performance.now()` deltas. Work only; waiting excluded. **This is therefore not directly comparable with the native `total_ms`, which is one continuous span.** The summary says so next to every web number.
- `completion_ms`: only in `variant=sync`. Immediately after the rAF callback's last render call, the host calls `gl.readPixels(0,0,1,1,…)` into a 1-pixel buffer (forces the ANGLE→SwiftShader pipeline to finish); `completion_ms = total_ms + readPixels duration`. In `variant=raf` it is empty. **Both variants are run and both are reported**; neither is relabelled as the other. The `sync` variant can distort the pipeline (forced stall), the `raf` variant under-counts asynchronous GPU-process work. The truth for this host lies between them.

### 2.3 Row emission rules
- Exactly one row per presented frame, including frames not caused by input (those are `event=idle_redraw` or `event=init`).
- The first frame is `event=init`, `frame_idx=0`.
- `event` vocabulary (closed set): `init, ptr_move, ptr_down, ptr_up, key_tab, key_char, key_other, ime_preedit, ime_commit, scroll_unused, resize, idle_redraw, other`. If a batch has mixed events, the row's `event` is the class of the **last** event; the host also increments a counter (§11) of events received so coalescing can be detected.
- The harness counts events it **sent** per scenario; "frames recorded for the scenario's measured classes" ÷ "events sent" is reported as `frame_per_event`. Below 1.0 means coalescing or no-damage events (reported, not corrected). A scenario whose `frame_per_event < 0.9` is flagged `coalesced` in the summary.
- Row writing must not take place inside the timed span of any frame (native: after `completion_ms` is sampled).

## 3. CSV schema (normative)

File `trace.csv` (native: written by the host to the path in env `PVC_TRACE`; web: rows pushed as objects with identical keys to `window.__pvcFrames`; the harness converts them to the same CSV). UTF-8, `\n`, header line exactly:

```
frame_idx,t_ns,scenario,event,update_ms,layout_ms,scene_ms,a11y_ms,mirror_ms,render_ms,total_ms,completion_ms
```

| Column | Type | Meaning |
|---|---|---|
| `frame_idx` | u64 | 0-based count of presented frames in this process/page |
| `t_ns` | u64 | native: `CLOCK_REALTIME` UNIX-epoch nanoseconds at `t_recv`. web: `round(performance.now()*1e6)` = nanoseconds since navigation start at `t_recv` |
| `scenario` | string | value of env `PVC_SCENARIO` / page query `?scenario=`; passed through by the host verbatim (the harness sets it; hosts do not interpret it) |
| `event` | string | closed vocabulary of §2.3 |
| `update_ms`…`render_ms` | float, 3 decimals | §2.2. Empty (not 0) when the stage does not exist on that path (`mirror_ms` on native). 0.000 when the stage exists but did no work |
| `total_ms` | float | §2.2 |
| `completion_ms` | float | §2.2; empty in web `variant=raf` |

Hosts may add **extra columns after `completion_ms`**; the harness ignores unknown ones. Hosts may **not** reorder or rename the first twelve.

Sidecar `meta.json` written once by the host at exit (native: path in env `PVC_META`; web: `window.__pvcMeta`): `{ "frames_presented": N, "events_received": M, "renderer": "<UNMASKED_RENDERER or wgpu adapter info>", "backend": "...", "vmhwm_kb": … (native only), "build": {"git_or_sha256": "...", "profile": "..."} , "limits": "downlevel_webgl2_defaults" | "default" , "navigator_gpu": "undefined" (web) , "mirror": "on|off" }`.

## 4. Per-path specifics

### 4.1 Native
- Process start: the harness records `t_spawn_ns = date +%s%N` (CLOCK_REALTIME) **immediately before `exec`** (`tools/measure/spawn.py` reads `time.time_ns()` and calls `os.posix_spawn`; spawn overhead below 2 ms is not subtracted).
- GPU completion: §2.2.

### 4.2 Web variants
`variant=raf` (no forced completion) and `variant=sync` (1×1 `readPixels`), selected by page query `?variant=raf|sync`. Both required (D-spec: SwiftShader completes asynchronously).

### 4.3 Mirror control
`?mirror=off` disables the mirror. Run as a control only, to compute the **mirror delta** (`mirror=on` minus `mirror=off` in total) as a cross-check on `mirror_ms`. The reported Q3 numbers (§5) use `mirror=on` rows. If dev-web does not provide the flag, the control is marked "not available".

### 4.4 Offscreen Tier 0 harness
If no windowed Tier 0 host exists, `tools/measure/bench_tier0` (my own tool, depends on `pvc_runtime` by read-only path) replays the §2.1 event scripts in-process (same event sequence and counts, no 50 ms gap needed) and renders with `vello_cpu` into a 1366×768 pixmap, writing the §3 CSV with `mode=offscreen`. `completion_ms = total_ms`.

## 5. Q3: mirror cost (reported two ways, always)
For every web cell, `mirror_ms` is reported (a) as an **absolute** p50/p95/p99/max in ms, and (b) as a **share of the web frame**: `Σ mirror_ms / Σ total_ms` over measured frames (ratio of sums), plus p50 and p95 of the per-frame ratio `mirror_ms/total_ms`. Both for `variant=raf` and `variant=sync`. Where the `mirror=off` control exists, also the paired difference of per-run median `total_ms`.

## 6. Idle
- Procedure: after the first frame is presented, wait **1 s** (settle), then observe **5 s** with **no input of any kind** (no mouse movement, no key, no window event from the harness). Specimen state: nothing focused (`idle-initial`).
- **Idle FPS = (frames presented in the 5 s window) ÷ 5.** Pass value is **exactly 0** (doc: §4.4.1 "0 FPS idle").
- Frames presented in the window are counted from two witnesses, and the larger is reported:
  1. rows in the trace whose `t_ns` falls in the window;
  2. an *independent* witness: web = an init script wraps `WebGL2RenderingContext.prototype.{drawArrays,drawElements,drawArraysInstanced,drawElementsInstanced,clear,clearBufferfv,readPixels}` and counts calls with `performance.now()` stamps (call count in window must also be 0); native = process CPU ticks (`/proc/<pid>/stat` utime+stime) and voluntary context switches over the window are recorded (a spinning loop would show as non-zero CPU; this witness is *indirect*, labelled so).
- Also reported, not part of the pass: `idle-focused` (F01 name field focused, caret visible, same 1 s + 5 s). A blinking caret that triggers redraws would make this non-zero; if so it is **flagged to the Orchestrator as a tension between "0 FPS idle" and a blinking caret**, not silently excused. The runtime may legitimately render a steady caret.
- Repetitions: 5 per specimen per path (per variant on web).

## 7. First frame
- Native: `first_frame_ms = (t_ns of row 0 + completion_ms of row 0) − t_spawn_ns` converted to ms. i.e. process spawn → first frame *completed on the GPU / presented*.
- Web: `first_frame_ms = (t_ns of row 0)/1e6 + completion_ms of row 0` in the `sync` variant (navigation start → first frame forced to completion), and `… + total_ms of row 0` in the `raf` variant. Navigation start is `performance.timeOrigin`; it **excludes** Chromium process launch and context creation, as a user-visible page-load number normally does. The harness also records Playwright's wall time `page.goto` → first row seen as a cross-check, labelled "includes CDP latency".
- Budgets (doc §4.4.1): **< 1 s** (Tier 1 paths: `gl-llvmpipe`, `chromium141-webgl2-swiftshader`), **< 1.5 s** (Tier 0: `tier0c-vello_cpu`). Pass = **max over the 10 repetitions** is below the budget **(decision)**; median and max are both printed.
- Repetitions: 10 fresh launches.

## 8. RSS (budget 150 MB, doc §4.4.1 rule 4)
- **Native:** `VmHWM` (peak resident set) of the host process, read from `/proc/<pid>/status` by the harness **while the process is still alive at the end of the scenario** (the host must stay alive until signalled, §11). Also recorded from the same file: `VmRSS`, `RssAnon`, `RssFile`, `RssShmem`. **What is included:** all resident pages of the one host process, including mapped shared libraries (Mesa/LLVM for llvmpipe, which is a software-rasteriser artefact absent on a real GPU), the font bytes, glyph caches, wgpu/vello buffers. **Not included:** Xvfb, dbus, the IME daemon (reported separately for information only). Budget comparison uses `VmHWM`; `RssAnon + RssShmem` is also shown because file-backed shared-library pages are shared with other processes.
- **Web:** at the end of the scenario, sum of `VmRSS` of all Chromium **renderer** and **GPU** processes (identified by `--type=renderer` / `--type=gpu-process` in `/proc/<pid>/cmdline`, descendants of the Playwright-launched Chromium), read from `/proc/<pid>/status`. Also reported separately: the browser process, the sum of per-process `VmHWM` (an upper bound on the peak; sum of peaks is not a simultaneous peak), and the sum of `Pss` from `smaps_rollup` (removes double counting of shared pages). **Method label:** `web-rss = Σ VmRSS(renderer+gpu), end of run`. Caveats stated next to the number: includes the browser's own baseline (V8, Blink, ANGLE, SwiftShader JIT); the budget is about the PVC runtime beside a browser, so this figure is an upper bound on PVC's own share. A baseline control (blank page, same Chromium flags) is measured once per path and subtracted **only in a separate, clearly labelled column**.
- Repetitions: 5 per specimen (the `rss` scenario); report max and median over repetitions.
- "150 MB" is ambiguous in the doc. **(decision)** The pass rule uses **150 MiB = 153 600 kB** (the more generous reading); the 150 000 kB reading is also printed so the reader can see which side of each a number falls on.

## 9. Controlled conditions, CPU lock, noise
- Each repetition is wrapped in `flock -x /workspace/orchestrator_work/pm/CPU_LOCK` (COMMON §4) for the whole span launch → teardown. No build, browser or LLM job of mine runs inside.
- At the start of every repetition the harness records `cat /proc/loadavg`, plus a **3 s CPU-idle probe** from `/proc/stat` (all 4 cores). If the probe shows < 90 % idle (someone is running outside the lock), the harness waits 20 s and re-probes, up to 6 times; if still noisy it runs anyway and sets `noisy=true` in that repetition's metadata, and SUMMARY.md prints such cells with a `⚠noisy` mark. It never silently drops or hides a noisy run.
- At the end of every repetition the probe is repeated for information (`idle_after`).
- Harness processes (xdotool, node/Playwright driver) run concurrently with the host on the same 4 cores; their CPU use is not subtracted. Estimated small (they sleep between events) but unmeasured; the harness records its own CPU time (`/proc/self/stat` of the driver) per repetition so the reader can see the order of magnitude.
- Xvfb and (native IME) fcitx5/ibus + dbus + AT-SPI registry are started by the harness **before** the lock-protected timed span of the first repetition and stay up between repetitions; they idle and are listed in the metadata with their PIDs.

## 10. Statistics
- **Warm-up discard (decision):** the first **10 measured-class frames of every repetition** are discarded from the percentile pools (glyph-atlas fill, JIT, shader/pipeline first-use). The discarded rows stay in the raw CSV and their max is reported as `warmup_max_ms` so the cold-start cost is not hidden.
- **Repetitions (decision):** interaction scenarios 5 fresh processes each; first-frame 10; idle 5; rss 5. Percentiles are over the **pool** of retained frames of all repetitions; per-repetition p95 is also listed (min/median/max across repetitions) to show run-to-run variance.
- **Percentile method:** linear interpolation between order statistics (NumPy `percentile(..., method="linear")`), over `completion_ms` (primary where present) and `total_ms` (always). p50/p95/p99/max and n are printed. No outlier removal.
- **No confidence interval is claimed on a percentile** in step 1; run-to-run spread is shown instead. (A bootstrap CI may be added in step 2 and will then be described here, dated.)

## 11. Hooks the hosts must provide (what the harness needs)

**Native (`pvc-native`):**
1. env `PVC_TRACE=<path>` → §3 CSV, flushed per row; env `PVC_SCENARIO=<id>` copied into the column.
2. env `PVC_META=<path>` → `meta.json` (§3) written at exit **and** on SIGTERM; process stays alive after the scripted work until it receives SIGTERM (the harness reads `/proc/<pid>/status` first).
3. env `PVC_RECTS=<path>` → JSON written after the first frame: `{ "<semantic id>": [x,y,w,h] }` in logical px for: L01 `connect_button`, `bio_text`; F01 `name`, `email`, `password`, `toggle`, `checkbox`, `submit`. The harness aims xdotool with these.
4. a way to choose the path at launch (`vello_cpu` windowed vs `gl` with downlevel limits) and a way to select spec/locale (`zh-CN`).
5. A frames-presented counter in `meta.json`.

**Web (`pvc-web`):**
1. `window.__pvcFrames` (array of §3-keyed objects), `window.__pvcMeta`, `window.__pvcRects` (same as native item 3, CSS px), query flags `?scenario=…&variant=raf|sync&mirror=on|off&specimen=L01|F01&locale=en|zh-CN`.
2. `window.__pvcReady` = true after the first frame presented.
3. No additional rAF loop when nothing is dirty (idle must be 0).

**Runtime (`pvc_runtime`):** per-frame stage timings (update, layout, scene, a11y) readable after each frame; renderer stage timing is host-side.

## 12. Re-run command (written in step 2; interface fixed now)
```
tools/measure/run.sh <path> <specimen> <scenario> [--variant raf|sync] [--mirror on|off] [--reps N] [--out DIR]
tools/measure/run_all.sh
```
`<path>` ∈ `tier0c-vello_cpu | gl-llvmpipe | chromium141-webgl2-swiftshader`. Output directory (default): `measurements/session2/<path>/<specimen>/<scenario>/` containing `trace_rep<k>.csv`, `meta_rep<k>.json`, `summary.json`, `summary.md`. One command regenerates the pooled `summary.json` of any cell from fresh runs; `tools/measure/summarize.py <dir>` regenerates summaries from raw CSV without re-running.

## 13. Pass / fail rules (**decision**, fixed before measuring)
The docs give budgets, not a statistic. The statistic is mine:

| Quantity | Budget (doc §4.4.1) | Pass rule used here | Also printed |
|---|---|---|---|
| Interaction frame, Tier 1 paths (`gl-llvmpipe`, `chromium141-webgl2-swiftshader`) | 16.6 ms | **p95 of `completion_ms` ≤ 16.6** (web `raf`: p95 of `total_ms`) | p50, p99, max; `strict` pass = max ≤ 16.6 |
| Interaction frame, Tier 0 (`tier0c-vello_cpu`) | 33 ms | p95 ≤ 33 | same |
| `gl-llvmpipe` additionally vs the hardware class | 33 ms | p95 ≤ 33 (informational: the path is the Tier 1 code path on Tier 0 (c)-class hardware) | |
| Idle | 0 FPS | frames in 5 s window = 0 (both witnesses) in **every** repetition | |
| First frame | < 1 s (Tier 1) / < 1.5 s (Tier 0) | max over 10 reps below budget | median |
| RSS | 150 MB | `VmHWM` (native) / web-rss (§8) ≤ 153 600 kB | the 150 000 kB reading, the anon+shmem figure, baseline-subtracted web figure |

A pass here means "passed on `container-xeon4` over this path", **never** "meets the Tier 1 budget". Both a pass and a fail are reported; a fail is not tuned away. If `completion_ms` is not available for a path/variant, the pass is stated on `total_ms` and says so.

## 14. Known limits of this method (stated in advance)
1. No vsync and no real compositor: native present under Xvfb is a CPU copy; web headless composition is not captured. Frame-time is *work per frame*, not displayed latency.
2. SwiftShader / llvmpipe frame time is CPU raster on 4 Xeon cores; it may fail budgets a real Tier 1 GPU would pass, or pass where a Celeron would not. No extrapolation is made.
3. The harness runs on the same cores as the host.
4. Percentiles over a few hundred frames per cell; p99/max are noisy.
5. Web `total_ms` is work-only and excludes the browser-process AX/compositor work after JS returns.
6. Interaction events are synthetic (xdotool/Playwright) but delivered through the real X11 / Chromium input paths, except IME-web (CDP) which is labelled.
7. Firefox/Chromium for the text bar (A8) is a separate method documented in `measurements/session2/text-bar/METHOD_TEXTBAR.md` (written before its measurements).
