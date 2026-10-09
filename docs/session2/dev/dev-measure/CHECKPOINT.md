# CHECKPOINT — dev-measure

## 2026-10-08 — METHOD.md v1 published (schema stable for hosts to code against)
Path: `/workspace/dev_work/dev-measure/METHOD.md`. No measurement exists yet.

**What hosts/runtime need from it (details in METHOD §3, §11):**
- CSV header, exactly: `frame_idx,t_ns,scenario,event,update_ms,layout_ms,scene_ms,a11y_ms,mirror_ms,render_ms,total_ms,completion_ms`
  - native: env `PVC_TRACE=<path>`, `PVC_SCENARIO`, `PVC_META=<path>` (meta.json at exit and on SIGTERM), `PVC_RECTS=<path>` (rects JSON after first frame). Process must stay alive until SIGTERM so the harness can read `/proc/<pid>/status` (VmHWM).
  - web: `window.__pvcFrames` (objects with the same keys), `__pvcMeta`, `__pvcRects`, `__pvcReady`; query flags `?scenario=&variant=raf|sync&mirror=on|off&specimen=&locale=`.
- `t_ns`: native = CLOCK_REALTIME epoch ns at event receipt; web = `performance.now()*1e6`.
- `total_ms`: native = event receipt → `present()` returned (one continuous span); web = WORK only (event-handler work + rAF callback work), waiting for the next tick excluded.
- `completion_ms`: native = receipt → forced GPU wait issued *after* sampling total_ms; web `variant=sync` = total + 1×1 `readPixels`; empty in `variant=raf`.
- `mirror_ms` (web): DOM mutation + one forced style/layout flush of the mirror subtree (`getBoundingClientRect()`); empty on native.
- one row per presented frame, including spontaneous ones (`event=idle_redraw`); `event` is a closed vocabulary (METHOD §2.3).
- No row writing inside a timed span.

**Interaction scripts are owned by me (METHOD §2.1)** and driven externally (xdotool / Playwright+CDP). Hosts do not need to implement scripted input beyond what they build for their own tests; they only need the hooks above (the dev-native/dev-web briefs mention scripted-input modes; I will use xdotool/Playwright directly unless you tell me yours are authoritative).

**Pass statistic (my decision, METHOD §13):** p95 of `completion_ms` ≤ 16.6 ms (Tier 1 paths) / 33 ms (Tier 0); strict = max. RSS 150 MiB.

**Ask for step 2:** when your hooks work, publish your own CHECKPOINT with the exact launch command and env/flags per path (incl. how to select `vello_cpu` windowed vs `gl` downlevel, specimen, `zh-CN`). I fill `tools/measure/paths.env` from that.

**Still missing (mine):** harness (`tools/measure/*`), runs, text bar. Step 2 starts after hosts report.
