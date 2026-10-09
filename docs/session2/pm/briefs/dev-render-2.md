# Brief: dev-render, second brief — headless `pvc-probe` for the pilot fidelity checker (B4-rt)

**Start only after you have reported Deliverable A (A2/A3/A7) and the Supervisor tells you to begin this brief.**
**Read first:**
- `/workspace/orchestrator_work/pm/briefs/COMMON.md` (still in force);
- `/workspace/dev_work/dev-baseline/PVC_CHECKER_NEEDS.md`. This is the contract. Use the latest version and its change log;
- `/workspace/dev_work/dev-ir/PILOT_COVERAGE.md` and `CHECKPOINT.md` (dev-ir's pilot AST);
- `docs/G4_PVC_IR_v0_Spec.md` §5, §7, §8.

**Serves:** Q1. The PVC arm's fidelity is judged through this probe, so it has to be faithful and no more generous than the baseline probe.

## Task
Supervisor decision D-2: extend `pvc_runtime`'s **headless** layout and behaviour execution to the constructs that the pilot specimens L01–L10 and G01–G05 need. Add the `pvc-probe` binary exactly as `PVC_CHECKER_NEEDS.md` §2 specifies: JSON lines on stdin and stdout; Snapshot per §1.
- **No rendering** for the new constructs. No GPU, no window, no network.
- The probe uses the **same** layout, text (Parley, bundled fonts) and semantics (`TreeUpdate` → PVC role names) code as the renderer. There is no second implementation of anything the renderer already has.
- Scope is exactly what `PVC_CHECKER_NEEDS.md` §4 lists, no more:
  - breakpoint reflow, grid with `fr` tracks, equal gaps, wrap;
  - direction-relative layout (RTL);
  - text scale;
  - disclosure/expanded, tabs, dialog layer;
  - `list<record>` ops, `decimal`, enum step state;
  - persisted `storage` with `new_session`/`reset` semantics as §2 defines them;
  - hit-tested click, focus order and Tab.
- Still a direct sugar solver (D-4).
- **Landmarks (Orchestrator decision):** L04's header/aside/footer use the `region` role with a name, which G4 §8.1 already defines. Do **not** add `banner`/`contentinfo`/`complementary` roles. This is recorded as a G4 §8.1 inconsistency for Deliverable C (AccessKit 0.25.1 has `Banner`, `Complementary`, `ContentInfo`; G4 omits them).
- If the checker needs something you consider beyond §4, or a Snapshot field the runtime cannot honestly compute, flag it to the Supervisor. Don't approximate silently.

## Done means
- `pvc-probe` passes dev-baseline's PVC checker validation: their hand-written pass and fail examples, run by dev-baseline. Coordinate through the Supervisor.
- A probe self-test per specimen using dev-ir's `pilot-refs/*.pvc`. Every op is exercised at least once.
- `cargo tree -e normal` shows no net/HTTP/LLM crate.
- A `REPORT.md` section "B4-rt", with checked and inferred kept separate, that lists every construct added and the specimen that needs it.

## Files you may touch
Only `/workspace/dev_work/dev-render/**`.
