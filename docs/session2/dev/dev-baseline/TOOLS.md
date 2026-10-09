# Baseline tool outputs for the agent (G2 §4.3 tool parity)

`baseline/bin/baseline-tool <html-validate|tsc|axe> <B1|B2> <file-or-dir>` prints plain text, first line = summary, then ≤ 15 problem lines, then `... N more`. Output is deterministic for a given input (no timestamps, no paths). No screenshots (G2 §4.3).

| tool | B1 | B2 | line format |
|---|---|---|---|
| `html-validate` 11.16.2 (`html-validate:recommended`, minus `no-inline-style`, `require-sri`, `no-trailing-whitespace`, `void-style`, `attribute-boolean-style`, `attribute-empty-style`, `prefer-native-element`, `no-implicit-close`, `tel-non-breaking`, `doctype-style`, `long-title`) | the file source | the **rendered DOM** after React mounts (`<script>/<style>/<link>` removed) | `LINE:COL error\|warn RULE: message` |
| `tsc` 5.9.3 (`strict`, `jsx: react-jsx`, `moduleResolution: bundler`, `noEmit`) | n/a ("not applicable to B1") | type-checks `App.tsx` (+ other files in the directory) | `FILE:LINE:COL TSxxxx message` |
| `axe-core` 4.14.0 (default rule set, violations only, page at 1280×800) | the built page | the built page after React mounts | `impact RULE: help \| N node(s) \| e.g. <html snippet ≤ 90 chars>` ordered critical→minor, then rule id |

`check-baseline` (the fidelity verdict) is separate: `baseline/bin/check-baseline <B1|B2> <ID> <file-or-dir> [--verbose] [--json] [--only IDs]`.
Exit codes: 0 = pass, 1 = fail, 2 = checker error. Default output (no `--verbose`) is exactly what an agent may see as repair feedback (G2 §4.4): a `RESULT` line, then one `FAIL <item id>: <standardised text>` line per failed A item (and `FAIL BUILD` if the program does not build). `--verbose` adds the measured detail for developers; **it must not be fed to the agent**.

Build conventions (offline; no network at check time):
* **B1**: one `.html` file. The Tailwind CDN `<script>` is removed and replaced by CSS compiled locally by the Tailwind v3.4.19 CLI from the classes in the file (the CDN is Tailwind v3). `tailwind.config = {…}` scripts are evaluated and passed to the CLI as the config; `<style type="text/tailwindcss">` blocks are passed as extra CSS. Any other external request is blocked and listed under `--verbose`.
* **B2**: `App.tsx` (default export `App`) — a single file, or a directory of `.ts/.tsx/.css` files. `tsc` type-check (errors = BUILD failure), esbuild 0.28.2 bundle, Tailwind CLI. Fixed library: **MUI** `@mui/material` 7.3.11 (+ `@emotion/react` 11.14.0, `@emotion/styled` 11.14.1, `@mui/icons-material` 7.3.11), React 19.3.0. This is the single "mainstream component library" of G2 §3.3, fixed for the pilot (deviation: G2 lets the agent choose).
* Checks run in Chromium 141 (Playwright 1.56.0, `/opt/pw-browsers`), viewport 1280×800 default, `127.0.0.1` only.
