# How L01 and F01 satisfy their G2 checklists *by construction*

"By construction" = the property is a field of the signed canonical module (so it is true for every runtime that loads it), and a test in `pvc-ir/tests/specimens.rs` asserts it on the **loaded signed artifact** (`.pvca`), not on the terse source. Solved-geometry and rendered-pixel versions of the layout items are the runtime's job (dev-render D3 tests); this file does not claim them.

## L01 — profile card (`L01.pvc`, signed `L01.pvca`)
| G2 item | A/H | How it holds | Test |
|---|---|---|---|
| Name bold and larger than title | A | `text "Jane Doe" bold size=14` → weight 700, size 14 px; title `muted size=12` → weight 400, size 12 px | `l01_checklist` |
| Avatar circular, left of name | A | `avatar` expands to a square box with `shape: circle`; it is child 0 of a `row` whose child 1 is the name column (row = main axis left→right in LTR) | `l01_checklist` |
| Button spans card width | A | button `w=Fill` in the card `col` (cross-axis `Fill` = stretch to the card's content box); card `w=Px(320)` | `l01_checklist` (field values); solved width = runtime D3 |
| Exact strings present | A | en strings table holds the five README strings verbatim (`Jane Doe`, `Product Designer`, `Building the future of spatial computing via GPU pipelines.`, `Connect`, `JD`) | `l01_checklist` |
| U1 names | A | button name = `Connect`; avatar name = text of "Jane Doe" (`Name::FromNode`) | `f01_u1_every_interactive_element_is_named_in_every_locale` (covers L01 too) |
| U3 contrast | A | every text/background pair in the painted tree ≥ 4.5:1 (≥ 3:1 for large text) | `u3_text_contrast_meets_aa_for_specimen_colour_pairs` |
| U2 keyboard | A | the only action is a `button` (focusable role, `press` handler) — keyboard activation is a runtime rule (Enter/Space) | not testable on the IR alone: runtime |
| U4/U5 (320 px, 200 % text) | A | fixed `w=320` is capped at the parent content width, text wraps; **not verified here** | runtime D3 at 320 px |
| Effect/capability | — | `connect_user` is declared, bound to the button's `press`, uses `net.main`, declared in the manifest (host `connect.example.invalid`, `.invalid` is reserved by RFC 2606). Never dispatched to a network | `l01_checklist` |

## F01 — sign-up form (`F01.pvc`, signed `F01.pvca`, en + zh-CN)
| G2 item | A/H | How it holds | Test |
|---|---|---|---|
| Submit disabled until all valid | A | the submit button's `disabled` is `not valid`, `derive valid = not is-empty(name) and is-email(email) and length(password) >= 8 and terms`; the form's `submit` handler is guarded by `valid` (Enter in a field cannot bypass it) | `f01_submit_disabled_until_all_valid`, `f01_submit_handler_is_guarded_and_sets_done` (reference evaluator over the signed artifact; each condition broken individually) |
| Email validated | A | `is-email` (exact rule: GRAMMAR.md §6.1); the email `field` shows its error exactly while `not is-empty(email) and not is-email(email)`; no error on an untouched field | `f01_email_validated_and_errors_announced`, `pred::tests::email_rule` |
| Show/hide toggles masking and is named | A | password input `masked = not show_pw`; the toggle button's press handler flips `show_pw`; the button has `pressed = show_pw`, role `button`, name "Show password" / "显示密码" | `f01_show_hide_toggles_masking_and_is_named` |
| Errors announced to AT | A | error text has role `status`, `live=polite`, is `show`n only while the input is `invalid`, and the input carries `error_message` → that node and `invalid` = same condition | `f01_email_validated_and_errors_announced` |
| U1 names | A | every interactive node (3 fields, toggle, checkbox, submit) has a name key present and non-empty in **both** locales | `f01_u1_every_interactive_element_is_named_in_every_locale` |
| zh-CN table complete | — | every key exists in both tables, every zh-CN text contains CJK characters (not a copy of English) | `f01_every_string_key_exists_in_both_locales_and_zh_is_actually_chinese` |
| U3 contrast | A | as L01, light theme | `u3_…` |
| No effects / no capabilities | — | F01 declares none; `submit` only sets `status = done` and shows "Account created" | `f01_submit_handler_is_guarded_and_sets_done` |
| U2/U4/U5 | A | keyboard operability and reflow are runtime properties | runtime |

Not claimed: any H (human) item (none in L01/F01), screen-reader behaviour, rendered pixels.
