//! Every diagnostic code in the A1 subset fires on a minimal bad input, with a message key, a
//! location and (where G4 §9.3 names one) a machine fix. Also: warnings do not block, and good
//! minimal inputs are clean.

use pvc_ir::diag::{Edit, Severity};
use pvc_ir::*;

const H: &str = "pvc 0.1 use std@0.1 theme dark\nlocales en\nfont ui 400 sha256=ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280\n";

fn diags(body: &str) -> Vec<Diagnostic> {
    let src = format!("{}{}", H, body);
    match compile_terse(&src) {
        Ok(c) => c.diagnostics,
        Err(d) => d,
    }
}
fn first<'a>(d: &'a [Diagnostic], code: &str) -> &'a Diagnostic {
    d.iter().find(|x| x.code == code).unwrap_or_else(|| panic!("no {code} in {:#?}", d.iter().map(|x| x.render()).collect::<Vec<_>>()))
}

#[test]
fn good_minimal_input_is_clean() {
    let d = diags("button \"Go\" on.press={ }\n");
    assert!(d.is_empty(), "{:?}", d.iter().map(|x| x.render()).collect::<Vec<_>>());
}

#[test]
fn parse_error_unterminated_block() {
    let d = diags("col {\n text \"x\"\n");
    let x = first(&d, "PVC-E-PARSE");
    assert_eq!(x.severity, Severity::Error);
    assert_eq!(x.message_key, "parse.expected");
    assert!(x.location.span.is_some());
    assert!(x.params.iter().any(|(k, _)| k == "expected"));
}

#[test]
fn parse_error_missing_header_and_bad_string() {
    let d = compile_terse("button \"x\"").unwrap_err();
    assert_eq!(d[0].code, "PVC-E-PARSE");
    let d = compile_terse("pvc 0.1 use std@0.1\ntext \"abc\n").unwrap_err();
    assert_eq!(d[0].code, "PVC-E-PARSE");
    assert_eq!(d[0].message_key, "parse.unterminated-string");
}

#[test]
fn schema_error_unknown_field_has_field_list_fix() {
    let d = diags("box colour=red\n");
    let x = first(&d, "PVC-E-SCHEMA");
    assert_eq!(x.message_key, "schema.unknown-field");
    assert!(x.fixes.iter().any(|f| matches!(&f.edit, Edit::FieldList { fields } if fields.contains(&"fill".to_string()))));
    // nearest-candidate fix
    assert!(x.fixes.iter().any(|f| matches!(&f.edit, Edit::Choose { .. })));
}

#[test]
fn schema_error_missing_required_field_and_bad_type() {
    let d = diags("text-input name=@n\nstrings en { n \"N\" }\n");
    assert_eq!(first(&d, "PVC-E-SCHEMA").message_key, "schema.missing-field");
    let d = diags("box w=wide\n");
    assert_eq!(first(&d, "PVC-E-SCHEMA").message_key, "schema.expected-size");
}

#[test]
fn unknown_kind_suggests_nearest_std_component() {
    let d = diags("buton \"x\"\n");
    let x = first(&d, "PVC-E-UNKNOWN-KIND");
    assert_eq!(x.message_key, "kind.unknown");
    match &x.fixes[0].edit {
        Edit::Choose { candidates } => assert_eq!(candidates[0], "button"),
        e => panic!("{e:?}"),
    }
}

#[test]
fn ref_errors_state_effect_string() {
    let d = diags("state { open: bool = false }\ntext \"x\" show=(opne)\n");
    let x = first(&d, "PVC-E-REF");
    assert_eq!(x.message_key, "ref.state");
    match &x.fixes[0].edit {
        Edit::Choose { candidates } => assert_eq!(candidates[0], "open"),
        e => panic!("{e:?}"),
    }
    let d = diags("button \"Go\" on.press=nothing\n");
    assert_eq!(first(&d, "PVC-E-REF").message_key, "ref.effect");
    let d = diags("text @nope\n");
    assert_eq!(first(&d, "PVC-E-REF").message_key, "ref.string");
}

#[test]
fn a11y_name_missing_or_empty() {
    let d = diags("button \"\" on.press={ }\n");
    let x = first(&d, "PVC-E-A11Y-NAME");
    assert_eq!(x.message_key, "a11y.name-missing");
    assert!(x.location.node.is_some());
    let d = diags("box role=button on.press={ }\n");
    assert_eq!(first(&d, "PVC-E-A11Y-NAME").params[0], ("role".into(), "button".into()));
    let d = diags("text-input bind=v\nstate { v: text }\n");
    first(&d, "PVC-E-A11Y-NAME");
    // an avatar with nothing to take a name from
    let d = diags("avatar \"JD\"\n");
    let x = first(&d, "PVC-E-A11Y-IMG");
    assert!(x.fixes.iter().any(|f| f.key == "mark-decorative"));
    // ... but next to a person's name it is named from that text, no error
    assert!(diags("row { avatar \"JD\"; text \"Jane\" }\n").is_empty());
}

#[test]
fn cap_undeclared_has_declare_fix() {
    let d = diags("button \"Go\" on.press=ping\neffect ping uses net.main\n");
    let x = first(&d, "PVC-E-CAP-UNDECLARED");
    assert_eq!(x.message_key, "cap.undeclared");
    assert!(x.fixes.iter().any(|f| f.key == "declare-capability"));
    // declaring it clears the error
    let d = diags("button \"Go\" on.press=ping\ncap net.main hosts=[\"h.invalid\"] methods=[\"POST\"]\neffect ping uses net.main\n");
    assert!(d.iter().all(|x| x.code != "PVC-E-CAP-UNDECLARED"));
}

#[test]
fn string_missing_for_declared_locale() {
    let src = "pvc 0.1 use std@0.1\nlocales en zh-CN\ntext @k\nstrings en { k \"Hello\" }\n";
    let d = compile_terse(src).unwrap_err();
    let x = first(&d, "PVC-E-STRING-MISSING");
    assert_eq!(x.message_key, "string.missing");
    assert!(x.params.contains(&("locale".into(), "zh-CN".into())));
    assert!(x.fixes.iter().any(|f| f.key == "add-string"));
}

#[test]
fn inline_string_in_multi_locale_module_is_string_missing() {
    let src = "pvc 0.1 use std@0.1\nlocales en zh-CN\ntext \"Hello\"\n";
    let d = compile_terse(src).unwrap_err();
    first(&d, "PVC-E-STRING-MISSING");
}

// ---- extra codes the subset needs to be safe
#[test]
fn unknown_token_type_id_action_and_system_font() {
    let d = diags("box fill=tael\n");
    assert_eq!(first(&d, "PVC-E-UNKNOWN-TOKEN").message_key, "token.unknown-color");
    let d = diags("box pad=huge\n");
    assert_eq!(first(&d, "PVC-E-UNKNOWN-TOKEN").message_key, "token.unknown-space");
    let d = diags("state { n: int = 0 }\ntext \"x\" show=(n)\n");
    assert_eq!(first(&d, "PVC-E-TYPE").message_key, "type.mismatch");
    let d = diags("state { t: text }\nbutton \"x\" on.press={ t = true }\n");
    first(&d, "PVC-E-TYPE");
    let d = diags("col { text \"a\" id=dup; text \"b\" id=dup }\n");
    first(&d, "PVC-E-ID-COLLISION");
    let d = diags("box role=button name=@n\nstrings en { n \"N\" }\n");
    first(&d, "PVC-E-A11Y-ACTION");
    // no font declared: warning only, not blocking
    let c = compile_terse("pvc 0.1 use std@0.1\ntext \"x\"\n").unwrap();
    assert_eq!(c.diagnostics[0].code, "PVC-W-SYSTEM-FONT");
    assert_eq!(c.diagnostics[0].severity, Severity::Warning);
}

#[test]
fn diagnostics_are_keys_not_prose_and_render_is_stable() {
    let d = diags("buton \"x\"\n");
    let line = d[0].render();
    assert!(line.starts_with("E PVC-E-UNKNOWN-KIND 4:1 kind.unknown "), "{line}");
    assert!(line.contains("kind=buton"));
    assert!(line.contains("fix=choose[choose:button"));
    // same input twice -> identical diagnostic list (D1)
    assert_eq!(d, diags("buton \"x\"\n"));
}

#[test]
fn hard_wall_validator_has_no_io() {
    // The validator and compiler take &str / &Module and return values: there is no path/URL/socket
    // argument anywhere in the public compile API. (The dependency-tree evidence is in cargo-tree-*.txt.)
    let _f: fn(&str) -> Result<Compiled, Vec<Diagnostic>> = compile_terse;
    let _g: fn(&Module) -> Vec<Diagnostic> = validate;
}
