# QUIET WINDOW — A6 timing matrix (declared by the Orchestrator)

**Why:** `flock -x` for measurements starves while a stream of `flock -s` jobs keeps arriving. The A6 matrix needs a mostly-exclusive window, which dev-measure estimates at 60–90 min (*estimate*).

**Protocol** (everyone checks for the marker file below):
1. **Pre-window: freeze builds.** dev-render, dev-native and dev-web finish their current build and post "measurement-ready: <binary path> sha256 <hash>" (through the Supervisor) for the binaries dev-measure will run: `pvc-native`, the `pvc-web` wasm and page, and the vello_cpu frame bench. After posting they start no new `flock -s` job until the window closes. dev-baseline and dev-pilot pause Playwright/cargo/model jobs too.
2. **Window open:** dev-measure creates `/workspace/orchestrator_work/pm/QUIET_WINDOW_OPEN` once all three "measurement-ready" posts are in, or once the Orchestrator tells it to go. While this file exists **nobody but dev-measure may start any `flock -s` job, cargo build, browser run or model run.** Reading, writing docs and editing source without building are fine.
3. **Inside the window:** dev-measure runs the A6 matrix under `flock -x`, against exactly the posted binaries (sha recorded per measurement). dev-measure may build its own harness only. If a host binary turns out to be broken, dev-measure stops and reports. Builders do not hot-fix inside the window.
4. **Window close:** dev-measure deletes `QUIET_WINDOW_OPEN` and posts "A6 window closed: <SUMMARY.md path>". Normal shared-lock work resumes.
5. **Golden comparison (A7)** is not a timing job, but it is CPU-heavy, so it runs after the window, not inside it.
6. A binary changed after the window means its measurements are re-run (in a later window) or labelled as "measured on binary <sha>, superseded".
