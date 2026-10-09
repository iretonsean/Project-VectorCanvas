# Supervisor spot-check: Tier 0 (c) frame bench (recorded by the Orchestrator)

**Status:** indicative spot-check. **This is not the A6 matrix**, which was never run. The figures were relayed by the Supervisor in a message to the Orchestrator during the session-2 wrap-up. The Supervisor's raw output files are **not** in this repo, so this file is a record of a relayed result, not a raw measurement.

- **Host / path:** `host=container-xeon4 / tier0c-vello_cpu`. Headless, no window or present. General constraint solve excluded (D-4). 1366×768, DSF 1.0.
- **Binary:** `frame_bench`, sha256 `a213611a74d350d81fd6d2c5a3d3a6386d42745b243b4d93bffaeb9254b522b4`. Built by dev-render; listed in `docs/session2/dev/dev-render/bin_SHA256SUMS`. Source: `crates/pvc-runtime/examples/frame_bench.rs`.
- **Conditions as relayed:** quiet box, load average 0.13. Run after the container restart.
- **Results as relayed:**

| scenario | total p50 (ms) | total p95 (ms) |
|---|---|---|
| F01 type_en | 12.26 | 15.26 |
| F01 zh-CN IME | 12.98 | 16.57 |
| idle | `needs_frame() == false` (no frame requested) | — |

- **Reading:** the Tier 0 budget is 33 ms (analysis §4.4.1). These numbers are within it *on this host*, for headless render-only frames, so input-to-present on a window is not included. Nothing here says anything about a Raspberry Pi 4, an X230-class laptop or any Tier 1 device.
- **Related file:** dev-render's own earlier run of a different binary build (sha256 `053fb5d5…`) was made at load average 5.39 and is in `../dev-render-frame_bench/summary.jsonl` (F01 type_en total p50 12.11 / p95 17.41 ms). dev-render labels it indicative and superseded by A6.
