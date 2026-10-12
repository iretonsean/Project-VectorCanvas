# Reference: David (@dzhng) on X, "delete as many layers of abstraction as possible"

Saved 2026-10-12 by the owner as reference material for PVC. Screenshot: the .jpg beside this file.
Source: https://x.com/dzhng (post about 8 h old when saved). Skills repo he names: https://github.com/dzhng/skills

## Text (transcribed)

my new AI native software engineering principle is to delete as many layers of abstraction as possible

at this point, all my webapps are written in rust and compiled to wasm, UI is rendered directly in html and/or raw webgpu calls

no react/threejs/nextjs or any other intermediary abstractions

all my effort goes into writing custom validation harnesses for each app that fully quantifies perf / usability / what "good" means for the coding agent so it can hillclimb it

every agent sessions are planned via my explore unknowns skill (github.com/dzhng/skills), so all major decisions are frontloaded, then I just let the agent rip for 12+ hrs

at this point I'm solo shipping products that I wouldn't even dare to dream of a year ago

## Points to keep

1. Fewer abstraction layers: agents write directly against the platform (Rust → WASM, plain HTML, raw WebGPU), no frameworks.
2. The human's main work is the validation harness: measurable definitions of performance, usability and "good" that an agent can optimise against.
3. Plan first: all major decisions are made up front (his "explore unknowns" skill), then the agent runs unattended for 12+ hours.

## How it relates to PVC (notes, not decisions)

- **Same direction as PVC's core idea.** PVC removes the browser engine and UI toolkit between the agent and the
  pixels (README "No UI toolkit, no DOM layout"). He removes the frameworks but keeps HTML and WebGPU; PVC goes one layer
  further and draws everything itself.
- **The validation harness is the human's main job.** PVC already has the validator and the measurement work
  (golden images, frame time, the G2 token benchmark). His point suggests treating these as the main product of each
  session: a harness that says in numbers what "good" is, so the agent can improve against it without the owner.
  This fits M1 (`docs/session3/PLAN.md`): the token benchmark is that kind of harness.
- **Front-load decisions, then long unattended runs.** Matches owner decision 6 (a written plan approved before code).
  His "explore unknowns" skill (https://github.com/dzhng/skills) may be worth reading as a way to write those plans.
- **Evidence.** This is one developer's report on X, not a measurement. PVC's rule still applies: claims are measured,
  not asserted.
