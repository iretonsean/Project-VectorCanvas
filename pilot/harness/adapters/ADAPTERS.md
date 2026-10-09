# Adapter contract (harness ⇄ dev-baseline / dev-ir tools)

The harness is tool-agnostic. `config*.json` names, per stack, a list of **tools** and one **checker**, as argv lists.
Placeholders in argv: `{artifact}` (path of the extracted file, e.g. `.../turn2/index.html`), `{dir}` (its directory), `{id}` (specimen id, e.g. `L03`).

## Tool (fed back to the agent, counted as "tool tokens")
* Prints its findings as plain text on stdout, in a compact stable format. Exit status is ignored (stderr is saved, never shown).
* The harness shows the agent: `<tool name> output:` + the stdout (at most 40 lines, each cut at 300 chars, then `... (N more lines omitted)`),
  or `(no findings)` if stdout is empty.
* Intended tools: PVC = `pvc-validate` (dev-ir); B1 = `html-validate`, `axe-core`; B2 = `tsc`, `axe-core` (dev-baseline).

## Checker (fidelity: G2 A-items + U1–U5; never shown raw to the agent)
* exit 0 = every A item and U1–U5 pass → run ends as `pass`.
* exit 1 = at least one item fails; stdout has one line per failing item: `FAIL<TAB><item-id><TAB><standard text>`.
  The item id and text must be the same string for the same checklist item whatever the stack (G2 §4.4).
* exit 2 = the artifact could not be built/loaded (PVC: validator error-severity diagnostics, runtime refuses to load; B1: page cannot load;
  B2: esbuild bundle or load failure). The harness then sends the standard BUILD text; stdout is ignored.
* Any other exit status aborts the run with a harness error (not a pass, not a fail).
* The checker never sees the model, and nothing here calls an LLM.
