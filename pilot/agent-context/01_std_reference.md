LAYOUT: `col` (vertical, default), `row`, `grid cols=N` (N equal columns, children stretch to their cell), `box`, `surface` (page root; optional).
Props: `w` `h` = number | `fill` | `hug`; `pad` = none xs sm md lg xl | number | [v,h] | [t,r,b,l]; `gap` (same tokens); `align` start|center|end|stretch (across); `justify` start|center|end|between (along); `fill` colour; `border` soft|normal|strong|none; `radius` sm|md|lg|full|number; `show=(bool expr)`; `id=name`.
Rules: in a row `w=fill` shares the leftover width, in a column it stretches; `h=fill` takes the leftover height; in a column a fixed `w` is capped at the parent's width; text wraps; a node is `hug` (content-sized) by default.
Responsive: `md.<prop>=` applies when the screen is <= 768 px wide, `sm.<prop>=` when <= 480 (props: dir cols gap pad align justify w h show). Examples: `grid cols=3 md.cols=1`, `row md.dir=col`, `box w=200 md.w=fill`, `box md.show=false`.
Colours: bg surface input-bg fg body muted border border-soft border-strong accent blue teal error focus ink-light ink-dark, or #rrggbb. Spacing xs sm md lg xl = 4 8 16 24 32. Contrast is checked: keep text readable on its background.

CONTENT
- `text "Hi" [bold] [muted] [body] size=14 color=tok heading=1..6 role=paragraph|label|status|alert w=fill`. Dynamic: `text value=(expr)`; string with holes: `text @k n=(expr)` where `strings en { k "{n} left" }` (`{{` `}}` literal braces). Decimals print with 2 digits.
- `card` (padded filled box), `avatar "AK" size=40 fill=blue alt="Ana"` (needs `alt=` or a text after it in the same parent, else `decorative`), `shape circle|rect|rounded w=16 h=16 fill=… alt="…"|decorative`.
- `button "Save" fill=none name="Accessible name" disabled=(expr) on.press={ … }`; `fill=none` = outlined. `on.press=effectname` fires an effect.
- `link "Help" href="https://example.com/x"` (needs `cap open-url.main urls=["https://example.com/"]`) or `link "More" on.press={ … }`; icon-only: `link name="Home" href="…" { shape circle decorative }`.
- Inputs: `field "Label" bind=var [kind=email|search|multiline] [required] help="…" error="…" error-when=(expr)`, `password-field "Label" bind=var toggle=boolvar toggle-name="Show"`, `text-input name="…" bind=var [variant=search|multiline|email]`, `checkbox "Label" bind=boolvar`, `switch "Label" bind=boolvar` (bare form: `switch name="…" bind=…`), `radio "Small" bind=enumvar value=variant` inside `col role=radio-group name="…"`. They update the bound state themselves.
- `form "Name" on.submit={ when valid; … } { … }`; a `button … submit` inside raises it.
- `list { item { … } }` (list / list-item roles); headings: `text … heading=N`.
- `tab "Name" selected=(expr) on.press={ … }` inside `row role=tab-list`; panels are `case` nodes with `role=tab-panel`.
- Landmarks: `box role=navigation|main name="…"`, `box role=region name="Header"` (header, sidebar, footer are regions with a name).
- `layer name="Title" role=dialog|alert-dialog show=(expr) { … }`: a dialog shown over the page while the expression is true.
- `slot on=expr { case value { … } case value { … } }`: shows the child of the matching case (enum variant, or `true`/`false`).
- `view listvar as=x where=(expr) { template }`: one copy of the template per list row (inside it `x.field` is that row's field).

ACCESSIBILITY (enforced): every interactive node has a name (labels give it); buttons/tabs/links need an action; images and shapes need `alt=` or `decorative`; region, radio-group, dialog, search and multiline inputs need `name=`.

BEHAVIOUR
State types: `bool` `int` `decimal` (2 digits, `4.99`) `text` `enum(a, b)` `list(f:type, g:type) max=N = [{ f=1 g="x" }]` (max required).
Handler `{ stmt; stmt }`: `field = expr` · `when expr` (guard: nothing runs if false) · `emit effect` · `append list { f = expr, g = expr }` · `remove x` (x = the view's row name) · `x.field = expr`. An event may have several `on.press={…}`; all run in order, each with its own guard. Events: `on.press` `on.change` (inputs) `on.submit`.
Expressions: `not and or == != < <= > >= + - *` (write `a - b` with spaces), `is-empty(x)` `length(x)` `contains(t, s)` `is-email(t)` `count(list)` `count(list, pred)` `sum(list, expr)` (inside, `it.field`) `percent(dec, 8.25)` (rounded to the cent); int*decimal is decimal; enum variants are bare words.
Every `effect` needs a matching `cap` line.
