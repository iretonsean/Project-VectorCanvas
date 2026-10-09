# Brief: dev-baseline — executable fidelity checklists for both stacks; baseline toolchain and checker (B4)

**Your name / directory:** `dev-baseline` → `/workspace/dev_work/dev-baseline/`
**Read first:** `/workspace/orchestrator_work/pm/briefs/COMMON.md` (mandatory); `docs/G2_Benchmark_Corpus.md` §3.1 (U1–U5), §3.2, §3.3, §4.3, §5 (the rows L01–L10 and G01–G05) and §7; `docs/G4_PVC_IR_v0_Spec.md` §8 (roles and the ARIA mapping).
**Serves:** Q1. A cheaper wrong result is a failure, so the token comparison is only as good as these checks.
**Independence rule:** you are deliberately *not* the author of the PVC agent context (that is dev-ir). Don't read `/workspace/dev_work/dev-ir/agent-context/` while writing the checklist specs. Write the specs from G2 alone.
**Phase now: prep only (D-6).** No pilot runs.

## 1. Checklist specs (stack-neutral), written first
`/workspace/dev_work/dev-baseline/checklists/<ID>.md` for each of the 15 pilot specimens (L01–L10, G01–G05). For each specimen:
- every G2 checklist item, marked A or H;
- for each A item, an **operational definition** in terms of observables both stacks can provide:
  - **geometry** at viewport widths 320, 480, 768, 1024, 1280 (and 200% text scale for U5);
  - the **accessibility tree** (role, name, states);
  - a **scripted interaction trace** (sequence of events, and expected state/visible-text/tree after each).
- H items are marked "not judged (D-5)".
- U1–U5 are defined the same way. U3 contrast: WCAG 2.2 AA ratios (4.5:1 text, 3:1 large text and UI components), computed from resolved colours.

These specs fix what "correct" means **before** any agent output exists. Write `checklists/INDEX.md` listing them with sha256 hashes, then tell the Supervisor "CHECKLISTS ready: <path>". They get committed before runs.

## 2. Baseline checker (Playwright, Chromium 141 at `/opt/pw-browsers`)
- **Toolchain, local npm in your directory:** react, react-dom, typescript, tailwindcss (CLI), esbuild, html-validate, axe-core, plus **one** mainstream React component library, chosen and named with its version. G2 §3.3 lets the agent choose a library. For the pilot, fix one pre-installed library and state that as a deviation. List every package with its version.
- `check-baseline <B1|B2> <ID> <file-or-dir>`: builds (B2: `tsc` type-check, esbuild bundle, Tailwind build; B1: Tailwind CDN is **not** allowed because there is no network at check time, so compile Tailwind locally), loads the result in Chromium offline, executes the spec's A items and U1–U5, and prints a **stable, compact** list of failing item IDs plus standardised text. dev-pilot feeds that back as repair feedback.
- Tool outputs for the agent (G2 §4.3 parity): `html-validate`, `tsc` errors (B2), and axe-core findings, each as a compact text format. Document it.

## 3. PVC checker
- `check-pvc <ID> <file.pvc>`: runs the same spec items against PVC through headless APIs. These are **validator** (dev-ir `pvc_ir`) for parse/validity/a11y rules, and **runtime** (dev-render `pvc_runtime`) for geometry, a11y tree and state traces.
- Those runtime APIs for the pilot constructs do not exist yet. They come in a later dev-render brief. **Write the PVC checker against an interface you specify** in `PVC_CHECKER_NEEDS.md`: the exact queries you need, e.g. `layout_at(viewport_w, text_scale) -> rects by node id`, `tree() -> roles/names/states`, `dispatch(event) -> state`, `visible_text()`. Publish that file early and tell the Supervisor. The Orchestrator uses it to brief dev-render.

## 4. Validation of the checkers (before any run)
For each specimen and each stack:
- one hand-written **passing** example and one **failing** example (fails at least one A item);
- the checker gets both right.

Record the results in `checker_validation.md`. For PVC, run this once the runtime API exists. Until then, mark it pending.

## Done means
- 15 checklist specs plus INDEX with hashes, announced.
- Baseline checker and tool outputs working offline, validated on pass/fail examples for all 15 specimens × B1/B2.
- `PVC_CHECKER_NEEDS.md` published.
- PVC checker written against it, with validation pending or done.
- `REPORT.md`, with checked and inferred kept separate and all npm package versions listed.

## Files you may touch
Only `/workspace/dev_work/dev-baseline/**`.
