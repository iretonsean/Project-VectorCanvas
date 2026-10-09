# PVC-IR terse grammar, v0.1 — L01 + F01 subset (`pvc_ir` 0.1.0)

Status: **implemented and tested** by `pvc-ir/` (every statement below that says "rejected" or "defaults to" has a test or is visible in `src/expand.rs`). It is the concrete grammar G4 deliberately left open ("not normative syntax"). It starts from the G4 §15 illustration and keeps its token economy. Part 2 (the pilot constructs, §11) only *adds* to it; construct → specimen map in `PILOT_COVERAGE.md`.

Scope: node kinds `surface`, `box`, `text`, `text-input`; `std` components `card`, `avatar`, `button`, `field`, `password-field`, `checkbox`, `form`; layout sugar; paint; semantics; behaviour (typed state, `press`/`change`/`submit`, guards, assignments, validity predicates); one effect kind with a `net` capability that is **never dispatched to a network**; strings table.

## 1. Two forms, one meaning (G4 P3)
* **Terse form** (this document): what an agent writes. Defaults omitted, `std` components, theme tokens, inline strings.
* **Canonical form** (§9): what the validator produces, hashes and signs. Fully explicit. `canonicalize(&Module) -> Vec<u8>`.
* `compile_terse` = parse (§2–§3) → expand (§5–§7) → validate (§10). Parsing never touches the network, a clock, or randomness (D1).

## 2. Lexical structure
* Source is UTF-8. Whitespace: space, tab, CR. **Newline ends a statement** (a node header, an item line, a `state` entry), except inside `( )` / `[ ]`, or when the line ends with `\` (continuation). `;` also ends a statement. Blocks are `{ … }`.
* Comments: `//` to end of line.
* Tokens:
  * **word**: `[A-Za-z_#][A-Za-z0-9_#]*` with `-`, `.`, `/`, `@` allowed *between* alphanumerics (`is-empty`, `on.press`, `std@0.1`, `zh-CN`, `#1e90ff`). Digits followed by letters are also a word (hex digests). **Consequence: write `a - b` with spaces; `a-b` is one word.**
  * **number**: `[0-9]+(\.[0-9]+)?`. Negative numbers: `-` then number (only in expressions and list/property values).
  * **string**: `"…"` with escapes `\" \\ \n \t \u{hex}`. No raw newline inside.
  * **key reference**: `@name` where name is `[A-Za-z0-9_.\-/:]+`.
  * punctuation: `{ } ( ) [ ] ; , = : == != < <= > >= + - * ..`
* Strings are normalised to **NFC** on input.

## 3. Syntax
```
module   := header NL item*
header   := 'pvc' NUMBER ['use' 'std@'NUMBER] ['theme' WORD] ['dialect' WORD]       // pvc 0.1 use std@0.1 theme dark
item     := locales | fonts | font | cap | state | derive | effect | strings | node
locales  := 'locales' WORD+                                                         // first = default (authoring) locale; default: en
fonts    := 'fonts' 'std'                                                           // the std@0.1 default font set (4 faces below, hashes of the bundled files); costs ~3 tokens instead of ~250
font     := 'font' WORD NUMBER { WORD '=' value }                                   // font ui 400 sha256=<64 hex> [file="…"] [index=N] [script=Hans]
cap      := 'cap' WORD { WORD '=' value }                                           // cap net.main hosts=["h"] methods=["POST"] [max-request=N] [max-daily=N]
state    := 'state' '{' (WORD ':' type ['=' literal] STMT_END)* '}'                 // type := bool | int | text | enum(WORD,…)
derive   := 'derive' WORD '=' expr                                                  // named pure expression, may use earlier names
effect   := 'effect' WORD 'uses' WORD                                               // effect connect_user uses net.main
strings  := 'strings' WORD '{' (WORD STRING STMT_END)* '}'                          // strings zh-CN { key "text" }
node     := WORD { arg | flag | WORD '=' value } [ '{' node* '}' ]                  // children separated by NL or ';'
arg      := STRING | KEYREF | NUMBER                                                // positional (the label)
flag     := WORD                                                                    // bare word, e.g. bold, muted, submit
value    := STRING | KEYREF | NUMBER | WORD | '(' expr ')' | '[' value {','value} ']' | '{' handler '}'
handler  := stmt*   stmt := 'when' expr | WORD '=' expr | 'emit' WORD              // separated by NL or ';'
expr     := or ;  or := and {'or' and} ;  and := not {'and' not} ;  not := 'not' not | cmp
cmp      := atom [('=='|'!='|'<'|'<='|'>'|'>=') atom]
atom     := 'true' | 'false' | NUMBER | '-'NUMBER | STRING | WORD | WORD '(' [expr {',' expr}] ')' | '(' expr ')'
```
Reserved item words (cannot be node kinds): `pvc locales fonts font cap state derive effect strings`. A bare word in an expression is a state field / derived name, or (if it is not one) a variant of some declared `enum` (`status == done`). Expression-valued properties take a parenthesised expression or a single word/number: `show=submitted`, `disabled=(not valid)`.

Top level: one `surface` node, **or** any number of nodes, which are wrapped in an implicit surface (§5.1).

## 4. Names, ids and strings
* **Node ids** are deterministic: root `surface0`; a child is `<parent-id>/<kind><n>` where `n` counts siblings of the same terse kind (`column` counts as `col`): `surface0/card0/row0/avatar0`. `id=<word>` on any node sets its id **absolutely** (stable under reordering). `std` expansions name their parts `<id>/label`, `/input`, `/help`, `/error`, `/row`, `/toggle`, `/box`, `/mark`. Duplicate ids → `PVC-E-ID-COLLISION`.
* **Strings table:** a string literal in a text slot (`text "Jane Doe"`) is moved to the **default locale's** table under the key `<node-id>:<slot>` (`surface0/card0/row0/col0/text0:t`; slots: `t`, `label`, `initials`, `name`, `alt`, `help`, `error`, `desc`, `toggle-name`). `@key` refers to an author-named key defined in `strings <locale> { … }`. A key used by the tree must exist in **every** declared locale: absent everywhere → `PVC-E-REF` (`ref.string`); absent in some declared locale → `PVC-E-STRING-MISSING` (so an inline literal in a multi-locale module is reported for each other locale). No plural/select forms in this subset.
* Keys in `strings` blocks are words (`signup.title`); they are opaque identifiers, not English (P4).

## 5. Node kinds and their properties
Common to all nodes where listed: `id`, `role=<role>`, `name=<str>`, `desc=<str>`, `live=off|polite|assertive`, flag `decorative`, `disabled=<expr>`, `show=<expr>`, `on.press|on.change|on.submit=<handler or effect-name>`. Layout props (`w h pad gap align justify`) and paint props (`fill border radius`) apply where listed. Unknown property → `PVC-E-SCHEMA` (`schema.unknown-field`) with the field list and nearest-name fixes. Unknown kind → `PVC-E-UNKNOWN-KIND` with nearest kinds.

### 5.1 Core kinds
| kind | arg | flags | properties | defaults (explicit in canonical) |
|---|---|---|---|---|
| `surface` | — | decorative | id role name desc live disabled show on.* w h pad gap align justify fill border radius | col, `w`=`h`=`Fill` (forced), pad 24, align center, fill `bg`, role container. Implicit surface = same. |
| `box`, `col`/`column`, `row` | — | decorative | same as surface | dir col (`row` → row), gap 0, pad 0, align start, justify start, w=h=hug, no paint, role container |
| `text` | label (string or `@key`), required | `bold` `muted` `body` `decorative` | id size color align heading w role name desc live disabled show on.press | size 14, weight 400 (`bold` → 700), colour `fg` (`muted` → `muted`, `body` → `body`, `color=<token>` wins), line-height = size × 1.4 (integer rounding), family `ui`, role `label` (`heading=N`, N=1..6 → role heading + level) |
| `text-input` | — | `required` `decorative` | id **bind** variant masked name desc live disabled show on.change on.submit w h pad gap align justify | variant `single` (`email`, `password`), masked = `true` for password else `false`, w=`Fill`, pad 12×8, fill `input-bg`, 1 px border `border`, radius 8, size 14, role text-field / email-field / password-field. **`name=` is required** (`PVC-E-A11Y-NAME`) |

`bind` names a `text` state field. `masked=(expr)` while true draws bullets.

### 5.2 `std@0.1` components (each expands deterministically into core kinds)
| component | usage | expands to |
|---|---|---|
| `card` | `card w=320 border=soft pad=lg gap=md { … }` — props as `box` | `box`: fill `surface`, radius 12, pad 16, gap 16, col, role container; `border=soft|normal|strong|none` (1 px, tokens `border-soft|border|border-strong`) |
| `avatar` | `avatar "JD" size=40 fill=blue` ; props `id size fill alt show`, flag `decorative` | 40×40 (`size`) **circle** box, role `image`, row centred, child text (initials, 40 % of size, bold, ink = whichever of `ink-light`/`ink-dark` has the higher contrast on the fill; decorative). **Accessible name:** `alt=<str>` if given; else the first non-decorative `text` found in the *following siblings* (the person's name; G4 §15); else `decorative`; else `PVC-E-A11Y-NAME` (fixes: set `alt`, mark `decorative`) |
| `button` | `button "Connect" w=fill fill=teal on.press=connect_user` ; props `id fill w name disabled show on.press`, flag `submit` | box role `button`, row centred, pad 16×10, radius 8, fill (default `accent`), name = label (or `name=`), child text (ink by best contrast, decorative). `submit` adds a press handler `Submit(<enclosing form id>)` (error if outside a form). `on.press=<effect>` is shorthand for `{ emit <effect> }` |
| `checkbox` | `checkbox "I agree" bind=terms` ; props `id bind disabled show on.press` | row (gap 8, pad 2 vertical) role `checkbox`, name = label, `checked`=`Var(bind)`, press handler `bind = not bind`; children: decorative 20×20 box (radius 4, 1 px border) holding a decorative 12×12 `accent` mark with `show=bind` (a filled square, not a tick glyph), and the decorative label text |
| `form` | `form @signup.title w=360 on.submit={ when valid; status = done } { … }` ; optional label arg, props `id name desc live disabled show on.submit w h pad gap align justify` | `box` role `form`, col, gap 16, named by the label arg. Raises/handles `submit` |
| `field` | `field @email.label bind=email kind=email required help=@email.help error=@email.error error-when=(<expr>)` ; `kind=text\|email`; props `id bind kind help error error-when show disabled on.change on.submit`, flag `required` | col (gap 4, w Fill): label text (14 bold) + `text-input` (role `text-field`/`email-field`, `labelled_by` the label, name = label, `required`, `invalid` = error-when, `described_by` help, `error_message` → error node) + help text (12 muted) + error text (12, token `error`, role `status`, `live=polite`, `show` = error-when). `error` and `error-when` must come together (`schema.error-needs-when`) |
| `password-field` | as `field` plus `toggle=<bool state> toggle-name=<str>` (both required), no `kind` | as `field`, input is `variant=password`, `masked = not <toggle>`; input + a **toggle button** (role `button`, named by `toggle-name`, `pressed`=`Var(toggle)`, press handler `toggle = not toggle`, secondary look: 1 px border, no fill) share a row |

Text inside controls (button/checkbox/avatar/toggle labels) is `decorative`: the control carries the name, so assistive technology does not read it twice.

## 6. Behaviour
* **State** (`state { … }`): types `bool`, `int` (i64), `text` (user-entered text), `enum(a, b, …)`. Initial values default to `false`, `0`, `""`, first variant. **No floats.** Type errors → `PVC-E-TYPE`.
* **derive** names a pure expression of state and *earlier* derived names; read it like a state field (cannot be assigned).
* **Events:** `press` (pointer click or Enter/Space on the focused control), `change` (a text-input's value changed; the runtime has already written `bind`), `submit` (raised by `Submit(id)` actions, and by Enter in a single-line input on the nearest ancestor that has a `submit` handler). Other G4 events are not in this subset.
* **Handler** `{ stmt; … }`: `when <expr>` = guard (several are ANDed; all guards are evaluated before anything runs); `field = <expr>` assignment (type-checked, state fields only, effects of earlier assignments are visible to later ones); `emit <effect>` queues the effect. A handler runs to completion, in order, with no loops or recursion (G4 §7.3 totality).
* **Effects:** `effect <name> uses <capability-id>`; the capability must be declared by a `cap` line with that id (`PVC-E-CAP-UNDECLARED`, fix = the `cap` line to add, which publish-time human confirmation must approve, G4 §9.4). **The runtime routes `emit` to a dispatcher stub that only logs; nothing in this subset opens a socket.** Capability kinds known: `net storage file.pick clipboard.write clipboard.read camera mic location notify open-url share`; a `net` cap needs `hosts` and `methods`. `.invalid` hosts (RFC 2606) are used by the specimens on purpose.
* **Expressions:** `and or not == != < <= > >=` (`< <= > >=` on `int` only), literals, state/derived names, enum variants, and the pure functions
  * `is-empty(text) -> bool` — zero characters (no trimming).
  * `length(text) -> int` — number of Unicode scalar values.
  * `contains(text, text) -> bool` — substring.
  * `is-email(text) -> bool` — the exact rule in §6.1.
  Reference semantics: `pvc_ir::eval` (`eval`, `run_handler`).

### 6.1 `is-email` — exact rule (ASCII only; not RFC 5322; no IDN; no trimming)
`s` is an email iff **all** hold:
1. exactly one `@`;
2. local part: 1–64 bytes from `A-Z a-z 0-9 ! # $ % & ' * + / = ? ^ _ ` { | } ~ . -`, not starting or ending with `.`, no `..`;
3. domain: ≤ 253 bytes, at least two `.`-separated labels, each 1–63 bytes of `A-Z a-z 0-9 -`, not starting or ending with `-`;
4. the last label is ≥ 2 bytes and all ASCII letters.
Tests: `pred::tests::email_rule` (accepts `a@b.co`, `jane.doe+x@example.com`; rejects `a@b`, `a@b.c`, `a@@b.co`, `.a@b.co`, `a..b@c.de`, `a@-b.co`, `a@b.c0`, `名前@b.co`, `a b@c.de`, …).

## 7. Layout, paint, semantics (what the properties mean)
* **Lengths:** numbers are logical px (decimals allowed, rounded to 1/64 px); spacing tokens `none xs sm md lg xl` = 0, 4, 8, 16, 24, 32 (valid for `pad`, `gap`); radius tokens `none sm md lg` = 0, 4, 8, 12 (`radius=full`/`circle` → circle). `pad=lg`, `pad=[v, h]`, `pad=[t, r, b, l]`.
* **Sizes** `w`/`h`: `fill`, `hug`, or a number. Layout is the direct stack solver (D-4; no Cassowary/general constraints): see CHECKPOINT.md for exact semantics (main/cross axis, `Fill`, fixed-width cap in a column).
* **Colours:** theme tokens `accent bg blue body border border-soft border-strong error focus fg ink-dark ink-light input-bg muted surface teal`, or `#rrggbb[aa]`. Themes `dark`, `light` (both defined in `src/theme.rs`; the chosen theme's whole palette is embedded in the canonical module). Unknown token → `PVC-E-UNKNOWN-TOKEN` with nearest names.
* **Roles** (`role=`): `button link checkbox switch text-field password-field email-field group container image label paragraph heading status alert form main navigation region`. No landmark roles beyond these in v0 (Orchestrator ruling): header/aside/footer are written as `region` + a name.
* **Names:** roles that need one (`button link checkbox switch *-field image region`) must have it unless `decorative` (`PVC-E-A11Y-NAME`); button/checkbox/switch also need a `press` handler (`PVC-E-A11Y-ACTION`).

## 8. Diagnostics output (`pvc-validate`)
One line per diagnostic, stable field order, values quoted (JSON-style) only when they contain whitespace/quotes:
```
<E|W|I> <CODE> <line>:<col|-> <message-key> [node=<id>] [path=<ir-path>] [<param>=<value> …] [fix=<fix-key>[<edit>] …]
```
`<edit>` is one of `set:<prop>=<terse value, unquoted>`, `add:<terse line, unquoted>`, `choose:<a|b|c>`, `fields:<a|b|c>`, `expect:<token class>`. The last line is `OK errors=0 warnings=N` (exit 0) or `FAIL errors=N warnings=N` (exit 1). Message keys, not English (G4 §9.1). Implemented codes: `PVC-E-PARSE PVC-E-SCHEMA PVC-E-UNKNOWN-KIND PVC-E-UNKNOWN-TOKEN PVC-E-REF PVC-E-ID-COLLISION PVC-E-TYPE PVC-E-CAP-UNDECLARED PVC-E-A11Y-NAME PVC-E-A11Y-ACTION PVC-E-STRING-MISSING PVC-W-SYSTEM-FONT`. The parser reports the **first** syntax error only; schema/validator errors are all reported, in a fixed order.

## 9. Canonical form (what is signed)
Compact JSON, UTF-8, **fixed field order** (the order written by `canon::module_to_val`), no whitespace, **integers only** (lengths in 1/64 px, colours as `[r,g,b,a]` in the theme and token names in nodes), strings NFC with RFC 8785-style minimal escapes, strings-table entries sorted bytewise by key, theme colours sorted by token name, capability host/method lists sorted. Provenance (id → source span) is **not** serialised. The reader accepts exactly this shape; `load_signed` re-serialises and refuses any difference. Top-level order: `header, manifest, theme, strings, state, derived, effects, tree`.
Signed artifact: `PVC-SIGNED 1 ed25519 <pubkey hex>\n<signature hex>\n<canonical bytes>`, signature over `"PVC-IR-v0.1-artifact\0" ‖ canonical bytes`.

## 10. Deviations from / additions to G4 (flagged, not silently fixed)
1. **Canonical layout is the explicit sugar record, not a constraint list** (D-4). G4 §5.2 says the validator expands sugar into constraints.
2. **`show=<expr>`** on any node is an addition: G4 has only `slot` for conditional presence, and `slot` is out of the L01/F01 subset. A one-variant `slot` would be its Part-2 generalisation.
3. **`derive`** (named pure expressions) is an addition to G4 §7 so a validity predicate is written once and used for `disabled`, guards and errors.
4. **`is-email`** is a function not in G4 §7.3's list (comparisons, `is-empty`, `length`, `contains`, enum tests); G2 F01 needs "email validated".
5. A **fixed `w` in a column is capped at the parent's content width**: G4 says nothing on this; it is what keeps U4 (no clipping at 320 px) true for a 320 px card.
6. No dialect pack is embedded (`header.dialect` is null unless the terse header names one).
7. Acknowledgements (G4 §3) and provenance in the canonical module are not implemented (A1 has no warnings that need acknowledging).
8. G4 §8.1 role list omits `banner`/`complementary`/`contentinfo` although AccessKit 0.25.1 has them (`Banner`, `Complementary`, `ContentInfo`); v0 keeps the G4 list per the Orchestrator ruling.

## 11. Part 2 additions (pilot subset L01–L10 + G01–G05; D-2)
Everything above is unchanged; the A1 artifacts are byte-identical. Added (full table with the specimen that needs each: `PILOT_COVERAGE.md`; agent-facing summary: `agent-context/01_std_reference.md`):
* **Kinds:** `grid cols=N`, `shape circle|rect|rounded`, `layer`, `slot on=<expr> { case <value> { … } }`, `view <list> as=<alias> [where=(expr)] { template }`; std `link`, `switch`, `radio`, `tab`, `list`/`item`; `text-input variant=search|multiline`; `field kind=search|multiline`.
* **Breakpoint overrides** on any node: `md.<prop>=…` (width ≤ 768 px), `sm.<prop>=…` (≤ 480 px; wins over md); props `dir cols gap pad align justify w h show`.
* **Text:** `text value=(expr)`; `text @key hole=(expr)` with `{hole}` in the string (`{{ }}` literal braces); every locale's string is checked against the supplied holes.
* **State:** `decimal` (2 fraction digits), `list(f:type, …) max=N [= [{ f=v … }, …]]` (max mandatory), `enum`. Literals: `4.99` (decimal, at most 2 fraction digits), `10` (int).
* **Expressions:** `+ - *`, `count(list[, pred])`, `sum(list, expr)` (rows bound as `it`), `percent(dec, p)`; `alias.field` for the bound row of a `view`. `a-b` is one word: write `a - b`.
* **Handlers:** `append <list> { f = expr, … }`, `remove <alias>`, `<alias>.<field> = expr`; **several `on.<event>={…}` on one node all run in order**, each with its own `when` guard.
* **Capability:** `cap open-url.main urls=["<prefix>", …]`; `link … href="…"` is an `OpenUrl` action checked against it (`PVC-E-CAP-UNDECLARED`, `PVC-E-CAP-SCOPE`).
* **Roles added:** `radio radio-group text-area search-field list list-item tab-list tab tab-panel dialog alert-dialog`. Landmarks stay `region` + name (Orchestrator ruling). `PVC-E-A11Y-IMG` now covers every `image` role node (avatar, shape): named or `decorative`.
* **Rules added:** `PVC-E-A11Y-IMG`, `PVC-E-A11Y-CONTRAST`, `PVC-E-UNBOUNDED`, `PVC-E-CAP-SCOPE`, `PVC-E-A11Y-ACTION` for link/tab/radio, `PVC-W-SLOT-INCOMPLETE`; list/row/alias/field `PVC-E-REF` and decimal/list `PVC-E-TYPE`.
* **Lexer:** several `strings` entries may share a line (`strings en { a "x"  b "y" }`).
* **Not added** (no specimen needs it; unsupported input → `PVC-E-UNKNOWN-KIND`/`PVC-E-SCHEMA` with a fix): image, scroll, wrap, timer, hover events, select/slider, accordion, `remove-where`/`update-where`/`sort-by`/`clear`, ICU plurals, raw constraints.
