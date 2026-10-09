# Brief: dev-native — native host: window, GL present, AccessKit/AT-SPI, system IME (A4)

**Your name / directory:** `dev-native` → `/workspace/dev_work/dev-native/`
**Read first:** `/workspace/orchestrator_work/pm/briefs/COMMON.md` (mandatory), then `docs/G6_Text_Strategy.md` §6, `docs/G4_PVC_IR_v0_Spec.md` §8, analysis §4.4.1, and FINDINGS F8, F9, F14 and F15. Code against `/workspace/dev_work/dev-render/CHECKPOINT.md` (the `pvc_runtime` API) once it exists. Until then, scaffold against your own placeholder.
**Serves:** Q2 (system IME on a native host), Q4 (frame budget with text and a11y live).

## What to build: crate `pvc_native` at `/workspace/dev_work/dev-native/pvc-native/`
1. **Window and present:**
   - winit 0.30.13 window at 1366×768 on X11 (Xvfb, e.g. `Xvfb :<n> -screen 0 1366x768x24`; use a display number of your own, not `:99`);
   - wgpu 30.0.1 **GL backend** surface on Mesa llvmpipe;
   - vello_gpu rendering via dev-render's GL path.
   
   Redraw only when `pvc_runtime::needs_frame()` is true, using `ControlFlow::Wait`, so idle is 0 FPS. Count presented frames.
2. **CLI:** `pvc-native <signed-artifact> [--locale en|zh-CN]`. The artifact is loaded through `pvc_ir::load_signed`, and the native host refuses a tampered artifact. Add a test using dev-ir's tampered fixture.
3. **Accessibility:** `accesskit_winit` 0.34.1, fed with `pvc_runtime`'s `TreeUpdate`, plus action handling (Click → press, Focus, SetValue/ReplaceSelectedText for text fields if supported).
   - **Verification by an external AT-SPI client:** install `at-spi2-core python3-pyatspi` (apt, allowed by D-9). Run a dbus session bus and the AT-SPI registry. Write a script `atspi_dump.py` (system `/usr/bin/python3`, which has pyatspi) that finds the PVC window and dumps roles, names and states.
   - **Done** = the dump shows every F01 interactive element with the correct role and accessible name. A checkbox state change (driven by a key press through xdotool) appears in a second dump. The F01 error message is exposed with live/polite or the AT-SPI equivalent. Save the dumps.
4. **System IME, zh-CN pinyin:**
   - Install `fcitx5 fcitx5-chinese-addons xdotool` (apt, allowed). If fcitx5 cannot work headless, try `ibus ibus-libpinyin` instead.
   - Configure XIM (`XMODIFIERS=@im=fcitx` or `@im=ibus`). Enable `window.set_ime_allowed(true)` on focus of a text field, and set the IME cursor area from `text_input_rect`.
   - Drive it with `xdotool`: type `nihao`, then select the first candidate.
   - **Done** = winit delivers `Ime::Preedit` and then `Ime::Commit("你好")` (or the engine's first candidate; record what it was), the preedit is drawn in F01's name field, and the committed text is in runtime state *and* in the AT-SPI dump. Save logs and a screenshot (Xvfb `xwd`, or a GL readback).
   - **If it cannot be made to work:** write down exactly what you tried, with the logs. That is an acceptable result. **Never substitute synthetic winit IME events and call it "system IME".** A synthetic-event test may exist, but it must be labelled synthetic.
5. **Instrumentation:**
   - per-frame CSV via env `PVC_TRACE`, using the schema in `/workspace/dev_work/dev-measure/METHOD.md`;
   - a frames-presented counter;
   - time to first frame from process start;
   - `VmHWM` at exit, written to the trace or a sidecar.
   
   dev-measure runs the actual measurements. **You provide the hooks and a scripted-input mode** (xdotool scripts for pointer move over the button, Tab traversal, typing 20 Latin characters, the IME sequence, the show/hide toggle, the checkbox) that dev-measure can call.

## Done means
Items 1–5 above. Plus:
- a `run_native.sh` that brings up Xvfb + dbus + AT-SPI + IME and runs a scenario end to end;
- `cargo tree -e normal` (native target) saved, with no HTTP/LLM/net crate;
- `REPORT.md`, with everything that is *not* a Tier 1 device result labelled. This is Linux/X11 under Xvfb, not Android or ChromeOS (PLAN V3, V4).

## Files you may touch
Only `/workspace/dev_work/dev-native/**`, plus the apt packages named above. List them with versions in your REPORT. You are the only Developer running apt. If anyone else needs a package, it comes through the Supervisor to you.
