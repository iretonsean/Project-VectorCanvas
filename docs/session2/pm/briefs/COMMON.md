# COMMON rules for every Session 2 Developer (read this before your own brief)

You are a Developer on Project VectorCanvas (PVC), Session 2. The Orchestrator wrote your brief. The Supervisor spawned you, relays your questions, and is the only one who pushes to the remote. **This session produces evidence, not a framework.** Build only what your brief asks for.

## 1. Read first, in this order
1. `/workspace/supervisor/SESSION2_BRIEF.md`: the session's goals, Supervisor rulings 1–6, red-team questions.
2. `/workspace/orchestrator_work/pm/PLAN.md`: the task breakdown. Your task IDs are in §3, deviations V1–V10 are in §5, and decisions D-1 to D-10 are in §6, now **ruled** (see §6 below).
3. `/workspace/orchestrator_work/pm/feasibility/FINDINGS.md`: what is already known about this container (F1–F20). Don't rediscover it.
4. The repo docs your brief names, under `/workspace/Project-VectorCanvas/docs/` (main @ 5fdd412). **The repo is read-only for you.** Only the Orchestrator integrates.

## 2. Where you work
- Your directory is `/workspace/dev_work/<your-name>/`. Every file you create lives there. **Never write outside it**, except for the environment installs your brief explicitly lists.
- **One author per file.** You may *read* other Developers' directories and depend on them through read-only Cargo `path =` dependencies. Never edit their files. If you need a change in someone else's file, say so in your report or ask through the Supervisor.
- A file in another Developer's directory may be half-written. Rely on it only after its author has published a `CHECKPOINT.md` or `REPORT.md` that covers it.
- **Cargo target dirs are shared (Supervisor infra ruling; disk quota is about 41 GB):** build with `CARGO_TARGET_DIR=/workspace/cargo-target` for native and `CARGO_TARGET_DIR=/workspace/cargo-target-wasm` for wasm32. The pins are identical, so dependencies compile once. Concurrent cargo runs serialise on the target-dir lock, which is expected. Don't keep a private `target/`. Delete yours after your first successful build into the shared dir. Avoid `cargo clean` on the shared dirs. Check `df -h /workspace` before large downloads.
- Use your own Python venv (`python3 -m venv /workspace/dev_work/<name>/.venv`) for any pip install. `tiktoken` 0.14.0 is already installed in the system `python3`.

## 3. Hard constraints (Supervisor rulings; these are not negotiable)
- **Hard wall:** no LLM call and no network call in any runtime or host crate (`pvc-ir`, `pvc-runtime`, `pvc-native`, `pvc-web`). No HTTP, socket or LLM client crate (reqwest, hyper, ureq, tokio::net, openai…) in their dependency trees. Verify with `cargo tree -e normal --target <t>` and record the result.
- **Pins (G7), exact:** `wgpu = "=30.0.1"`, `vello_cpu = "=0.3.0"`, `vello_gpu = "=0.3.0"`, `parley = "=0.11.1"`, `fontique = "=0.11.1"`, `harfrust = "~0.12.0"` (resolves 0.12.0), `skrifa = "~0.44.0"`, `accesskit = "=0.25.1"`, `accesskit_winit = "=0.34.1"`, `winit = "=0.30.13"`, `glifo = "=0.4.0"`. Toolchain: cargo/rustc 1.97.0 is installed and the floor is 1.89.
- **Any Cargo dependency of a shipped crate outside that table** needs a one-paragraph RFC note at `/workspace/dev_work/<name>/rfc/<crate>.md` (what it is, exact version, why nothing pinned covers it, what it pulls in, and a hard-wall check). These are pre-approved (D-10): `ed25519-dalek` + `sha2` in pvc-ir; `wasm-bindgen =0.2.129`, `web-sys =0.3.106`, `js-sys =0.3.106` as direct deps of pvc-web. **Anything else: ask first.** Tooling and test-only dependencies (dev-dependencies, harness scripts, npm/pip tools) need no RFC, but you must list each one with its version in your REPORT.
- **No author shaders.** No LLM in the runtime.
- **Numbers:** every number you report comes from a file you produced, and you cite its path. Estimates are labelled *estimate*. Never present a target as a result. Every timing or memory number carries its host label (§5).
- **Flag, don't fix:** if you find an inconsistency in the repo docs or in another Developer's output, report it. Don't silently work around it.
- **No web-searching unknowns into claims.** One unknown in particular stays *unverified*: Crostini GPU acceleration and ChromeVox bridging on Chromebooks.

## 4. CPU lock (the container has 4 cores, shared by everyone)
Lock file: `/workspace/orchestrator_work/pm/CPU_LOCK`.
- **Timing or memory measurements** take it exclusively: `flock -x /workspace/orchestrator_work/pm/CPU_LOCK <cmd>`.
- **Heavy jobs** (cargo build/test, browser runs, llama-server/model runs) take it shared: `flock -s /workspace/orchestrator_work/pm/CPU_LOCK <cmd>`. Many shared holders can run at once. They block only while a measurement holds the lock, and a measurement waits until they finish.
- Record `cat /proc/loadavg` at the start of every measurement, in its output file.

## 5. Host label (put it on every timing/RSS/FPS number)
`host=container-xeon4` (4 vCPU Intel Xeon @ 2.10 GHz, 15 GB, Ubuntu 24.04.5, no GPU), plus the path:
- `tier0c-vello_cpu`: vello_cpu, CPU.
- `gl-llvmpipe`: wgpu GL on Mesa 25.2.8 llvmpipe with WebGL2 downlevel limits imposed. This is the Tier 1 code path on Tier 0 (c)-class hardware.
- `chromium141-webgl2-swiftshader`: Chromium 141.0.7390.37 headless, `navigator.gpu` hidden, WebGL2 via ANGLE→SwiftShader (CPU).

**Nothing may be written as if it ran on a phone or a Chromebook.**

## 6. Supervisor decisions in force
- D-1: the pilot arms are Qwen3-4B Q4_K_M (≤4B arm, thinking off) and Qwen2.5-Coder-7B-Instruct Q4_K_M. Label everywhere: "both arms small local models; no hosted/frontier arm run; result speaks only for that class."
- D-2: the headless validator/checker may cover the pilot specimens' node kinds (no rendering). This is the one build beyond L01/F01.
- D-3: the pilot specimens are L01–L10 and G01–G05, in an interleaved order fixed in the pre-registration. If compute binds, the analysed set is the completed prefix of that order, and never less than L01–L05 + G01–G05.
- D-4: a direct sugar layout solver, no general Cassowary solve. State that exclusion next to every frame number.
- D-5: H checklist items are reported as "not judged". Pass is decided on A items only, and labelled as such.
- D-6: pilot prep may run in parallel with Deliverable A. **No pilot runs** until the Orchestrator says so.
- D-7: the G2 decision rule is applied to each arm individually.
- D-8: the falsification trigger is applied as written, with n and arms stated beside the statement.
- D-9: apt/npm/pip/tool installs listed in your brief are allowed. List each one with its version in your REPORT.

## 7. Shared interface conventions (so parallel work fits together)
- Crate names and locations:
  - `pvc_ir` at `/workspace/dev_work/dev-ir/pvc-ir/`
  - `pvc_runtime` at `/workspace/dev_work/dev-render/pvc-runtime/`
  - `pvc_native` at `/workspace/dev_work/dev-native/pvc-native/`
  - `pvc_web` at `/workspace/dev_work/dev-web/pvc-web/`
- Each crate's author publishes `/workspace/dev_work/<name>/CHECKPOINT.md` as soon as its public API is stable enough for others to code against: the signatures, an example, and what is still missing. Update it (append, dated) when the API changes.
- **Semantics are produced once**, by `pvc_runtime`, as `accesskit::TreeUpdate`. The native host feeds it to `accesskit_winit`. The web host converts it to the hidden DOM/ARIA mirror. Nobody builds a second semantics model.
- **Frame instrumentation:** `pvc_runtime` exposes per-stage timings for the last frame (update, layout, scene, a11y, render, total). Hosts write per-frame rows to the CSV named by env var `PVC_TRACE` (native) or push them to `window.__pvcFrames` (web), using the schema in dev-measure's `METHOD.md` once that exists.
- **Primary viewport:** 1366×768 at device scale 1.0 (Tier 1 laptop/Chromebook panel-native, §4.4.1). The CJK locale is **zh-CN** (Simplified Chinese); IME is pinyin.

## 8. Reporting
- When done (or blocked), write `/workspace/dev_work/<your-name>/REPORT.md` with these sections:
  1. **Summary** (≤10 lines).
  2. **Done criteria:** each criterion from your brief marked met / partly met / not met, with the evidence file path.
  3. **Checked vs inferred:** two separate lists. *Checked* means you ran it and here is the file. *Inferred* means you reasoned it or it is an estimate.
  4. **Measurements:** paths and host labels, if any.
  5. **Deviations and labelled gaps.**
  6. **Dependencies:** Cargo additions with their RFC note paths, plus tooling and installs with versions.
  7. **Inconsistencies flagged:** in the docs or in other Developers' work.
  8. **Open issues.**
- Then report to the Supervisor with the path to your REPORT.md and a 3-line summary.
- **Ask rather than guess.** If something is ambiguous, send the Supervisor a one-line question, and it will be relayed to the Orchestrator.
