# dev-baseline — fidelity checklists and checkers (Session 2, task B4)

```
checklists/        15 frozen checklist specs (L01–L10, G01–G05) + U.md + INDEX.md (sha256)   ← generated from core/specs/*.mjs
core/              stack-neutral: Snapshot helpers (lib.mjs), per-item context (ctx.mjs), U1–U5 (universal.mjs), specs/, runner.mjs, gen-checklists.mjs
baseline/          Chromium/offline checker: build.mjs (B1/B2 builds), probe-browser.mjs (Snapshot from DOM+AX tree), check.mjs, tools.mjs,
                   bin/check-baseline, bin/baseline-tool, validate.mjs (checker validation)
pvc/               check-pvc, probe-pvc.mjs (client of the pvc-probe protocol), mock-probe.mjs (protocol conformance mock, NOT PVC)
examples/          B1/, B2/ hand-written passing + failing examples; mutations.mjs; make-fails.mjs; PVC/ (pending)
PVC_CHECKER_NEEDS.md   what pvc_runtime / pvc_ir must provide
TOOLS.md           tool outputs and build conventions for the baseline agent
checker_validation.md  results of validating the checkers
```

Design in one paragraph: every A item is a predicate over a JSON **Snapshot** (nodes with role/name/states, document-coordinate rects, text lines, resolved colours) plus a tiny event vocabulary (click / type / key / new session). The same predicate code runs on both stacks; only the Snapshot producer differs (Chromium DOM + accessibility tree for the baseline, `pvc-probe` for PVC). H items are "not judged" (D-5). Pass ⇔ all specimen A items and U1–U5 pass and the program builds/validates.

Run: `baseline/bin/check-baseline B1 L01 examples/B1/L01.pass.html --verbose` · `baseline/bin/baseline-tool axe B2 examples/B2/G03.pass.tsx` · `node baseline/validate.mjs [regex]` · `node core/gen-checklists.mjs` (regenerate checklists + INDEX).
Hold the CPU lock shared (`flock -s /workspace/orchestrator_work/pm/CPU_LOCK …`) for any run.
