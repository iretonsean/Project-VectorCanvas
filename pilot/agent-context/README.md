# agent-context/ (B2) — what is sent to the model and what is not

**Sent (the PVC "context", system message = these three files joined with `"\n\n"`, in this order, as `dev-pilot/harness/pilot.py::context_text` does):**
1. `00_card.md` — the agent card: task/output format, the validator's output format, program skeleton, syntax.
2. `01_std_reference.md` — the `std` reference: layout, content components, accessibility rules, behaviour and expressions.
3. `02_dialect_en.md` — the en dialect pack (English phrases → constructs) and three worked examples that are **not** from the G2 corpus (a music-player card, a sound panel, a song queue).

**Not sent:** this `README.md`, `FROZEN.md`.

Design rules followed (Q1): one card for every arm, no per-intent tuning (no worked example is L01–L10/G01–G05; `tests/agent_context.rs` checks that no example *is* a pilot reference program); no accessibility prose beyond the language's own enforced rules; compact, because its size is the fixed overhead PVC is charged per turn.
The examples and every documented syntax form are compiled by `pvc-ir/tests/agent_context.rs` so the text cannot drift from the validator.

## The validator tool (`pvc-validate`)
```
pvc-validate <file.pvc>            # text diagnostics, one per line (what the harness feeds back, counted as tool tokens)
pvc-validate --json <file.pvc>     # {"ok":…,"errors":N,"warnings":N,"diagnostics":[{code,severity,line,col,node,path,message_key,params,message,fix_hint,text}]}  (for checkers)
```
Exit status 0 = no errors (warnings allowed), 1 = at least one error, 2 = cannot read the file. No network, no clock, no randomness: the same file always gives byte-identical output (D1). The text form is
```
<E|W|I> <CODE> <line>:<col or -> <message-key> [node=<id>] [path=<ir path>] [<param>=<value> …] [fix=<fix-key>[<edit>] …]
…
OK errors=0 warnings=<N>            (exit 0)       or       FAIL errors=<N> warnings=<N>   (exit 1)
```
* Values containing whitespace or quotes are double-quoted (`\"`, `\\`, `\n` escapes). `fix=` payloads are bracketed and unquoted.
* `<edit>`: `set:<prop>=<terse value>` · `add:<terse program line>` · `choose:<a|b|c>` · `fields:<a|b|c>` · `expect:<token class>`.
* The parser stops at the **first** syntax error (`PVC-E-PARSE`), so a syntax error line is followed directly by `FAIL`. Schema/validator errors are all reported, in a fixed order (global checks, nodes in document order, strings).
* Codes: `PVC-E-PARSE SCHEMA UNKNOWN-KIND UNKNOWN-TOKEN REF ID-COLLISION TYPE UNBOUNDED CAP-UNDECLARED CAP-SCOPE A11Y-NAME A11Y-IMG A11Y-ACTION A11Y-CONTRAST STRING-MISSING`, warnings `PVC-W-SYSTEM-FONT PVC-W-SLOT-INCOMPLETE`. Messages are keys + parameters, never English prose (G4 §9.1).
* A program that compiles with **no error** is what the runtime would load. It can still fail geometry-based checklist items (U4/U5 etc.), which only the probe/checker can see.
