//! The agent context (agent-context/*.md) makes claims about the language. Every example program in it must
//! validate clean, and every syntax form it documents must compile. (Frozen text: if one of these fails, the
//! context is wrong, not the test.)

use pvc_ir::*;

fn ctx_dir() -> String {
    format!("{}/../agent-context", env!("CARGO_MANIFEST_DIR"))
}
const H: &str = "pvc 0.1 use std@0.1 theme light\nfonts std\n";
fn ok(body: &str) {
    match compile_terse(&format!("{}{}", H, body)) {
        Ok(c) => assert!(c.diagnostics.is_empty(), "{body}\n{:?}", c.diagnostics.iter().map(|d| d.render()).collect::<Vec<_>>()),
        Err(d) => panic!("{body}\n{:?}", d.iter().map(|x| x.render()).collect::<Vec<_>>()),
    }
}

#[test]
fn every_example_program_in_the_context_is_valid_and_not_a_corpus_specimen() {
    let mut n = 0;
    for f in ["00_card.md", "01_std_reference.md", "02_dialect_en.md"] {
        let t = std::fs::read_to_string(format!("{}/{}", ctx_dir(), f)).unwrap();
        let mut rest = t.as_str();
        while let Some(i) = rest.find("```pvc\n") {
            let after = &rest[i + 7..];
            let j = after.find("```").unwrap();
            let prog = &after[..j];
            let c = compile_terse(prog).unwrap_or_else(|d| panic!("{f}: {:?}", d.iter().map(|x| x.render()).collect::<Vec<_>>()));
            assert!(c.diagnostics.is_empty(), "{f}: {:?}", c.diagnostics.iter().map(|x| x.render()).collect::<Vec<_>>());
            // no example may be a pilot-reference program (hash of canonical bytes)
            let canon = canonicalize(&c.module);
            for id in ["L01", "L02", "L03", "L04", "L05", "L06", "L07", "L08", "L09", "L10", "G01", "G02", "G03", "G04", "G05"] {
                let r = std::fs::read_to_string(format!("{}/../pilot-refs/{}.pvc", env!("CARGO_MANIFEST_DIR"), id)).unwrap();
                assert_ne!(canon, canonicalize(&compile_terse(&r).unwrap().module), "{f} contains {id}");
            }
            n += 1;
            rest = &after[j + 3..];
        }
    }
    assert_eq!(n, 3);
}

#[test]
fn documented_diagnostic_line_has_the_documented_shape() {
    let d = compile_terse("pvc 0.1 use std@0.1\nfonts std\nbutton \"\" on.press={ }\n").unwrap_err();
    let line = d[0].render();
    assert!(line.starts_with("E PVC-E-A11Y-NAME 3:1 a11y.name-missing node="), "{line}");
    assert!(line.contains(" path=sem.name role=button fix=set-name[set:name=\"<name>\"]"), "{line}");
}

#[test]
fn documented_layout_and_content_forms_compile() {
    ok("state { v: text }\nbox w=100 h=fill pad=[sm, md] gap=lg align=center justify=between fill=surface border=soft radius=full show=true id=x { text \"a\" }\n");
    ok("grid cols=3 md.cols=1 sm.gap=xs { text \"a\" }\nrow md.dir=col { text \"a\" }\nbox w=200 md.w=fill { text \"a\" }\nbox md.show=false { text \"a\" }\n");
    ok("surface pad=lg { col gap=md { text \"Hi\" bold muted size=14 color=body heading=2 w=fill } }\n");
    ok("text \"Hi\" role=paragraph\ntext \"Hi\" role=status\ntext \"Hi\" role=alert\n");
    ok("card { text \"a\" }\navatar \"JD\" size=40 fill=blue alt=\"Jane\"\nshape circle w=16 h=16 fill=accent alt=\"Dot\"\nshape rect decorative\nshape rounded decorative\nrow { avatar \"JD\"; text \"Jane\" }\navatar \"JD\" decorative\n");
    ok("state { on: bool }\nbutton \"Save\" fill=none name=\"Save it\" disabled=(on) on.press={ on = true }\n");
    ok("cap net.main hosts=[\"api.example.invalid\"] methods=[\"POST\"]\neffect send uses net.main\nbutton \"Go\" on.press=send\n");
    ok("cap open-url.main urls=[\"https://example.com/\"]\nlink \"Docs\" href=\"https://example.com/x\"\nlink name=\"Home\" href=\"https://example.com/\" { shape circle decorative }\n");
    ok("state { b: bool }\nlink \"More\" on.press={ b = true }\n");
    ok("state { v: text; ok: bool; s: bool; e: enum(a, b) = a }\nfield \"Label\" bind=v required help=\"h\" error=\"bad\" error-when=(is-empty(v))\nfield \"Find\" bind=v kind=search\nfield \"Bio\" bind=v kind=multiline\nfield \"Mail\" bind=v kind=email\npassword-field \"Pw\" bind=v toggle=s toggle-name=\"Show\"\ntext-input name=\"Q\" bind=v variant=search\ntext-input name=\"Q2\" bind=v\ncheckbox \"Label\" bind=ok\nswitch \"Label\" bind=ok\nswitch name=\"L\" bind=ok\ncol role=radio-group name=\"Pick\" { radio \"A\" bind=e value=a; radio \"B\" bind=e value=b }\n");
    ok("state { v: text; ok: bool }\nform \"Sign in\" on.submit={ when ok; ok = false } { field \"V\" bind=v; button \"Go\" submit }\n");
    ok("list { item { text \"a\" }; item { text \"b\" } }\n");
    ok("state { t: enum(a, b) = a }\nrow role=tab-list { tab \"A\" selected=(t == a) on.press={ t = a }; tab \"B\" selected=(t == b) on.press={ t = b } }\nslot on=t { case a role=tab-panel { text \"x\" }; case b role=tab-panel { text \"y\" } }\n");
    ok("box role=navigation name=\"Main\" { text \"a\" }\nbox role=main { text \"a\" }\nbox role=region name=\"Header\" { text \"a\" }\n");
    ok("state { o: bool }\nlayer name=\"Title\" role=dialog show=(o) { text \"x\"; button \"Ok\" on.press={ o = false } }\n");
    ok("state { f: bool }\nslot on=f { case true { text \"y\" }; case false { text \"n\" } }\n");
}

#[test]
fn documented_behaviour_forms_compile() {
    ok("state { b: bool; i: int = 3; d: decimal = 4.99; t: text = \"x\"; e: enum(p, q) = p; l: list(f:int, g:text) max=5 = [{ f=1 g=\"x\" }] }\nbutton \"x\" on.press={ b = not b; i = i + 1 * 2 - 1; d = d + 1.00; t = \"y\"; e = q; append l { f = i, g = t }; emit_none = 1 }\n".replace("; emit_none = 1", "").as_str());
    ok("state { l: list(f:int) max=5; n: int; d: decimal = 1.00; t: text }\nderive total = sum(l, it.f * 2)\nderive c = count(l)\nderive c2 = count(l, it.f > 1)\nderive pc = percent(d, 8.25)\nderive pi = percent(d * 2, 10)\nderive q = not is-empty(t) and length(t) >= 2 or contains(t, \"a\") or is-email(t)\ntext value=(total)\n");
    ok("state { l: list(f:int, ok:bool) max=5 }\nview l as=x where=(x.f > 0 and not x.ok) { row { text value=(x.f); button \"d\" name=\"Del\" on.press={ remove x } ; checkbox name=\"ok\" bind=x.ok; button \"u\" on.press={ x.f = x.f + 1 } } }\n");
    ok("state { a: bool }\nbutton \"x\" on.press={ when a; a = false } on.press={ when not a; a = true }\n");
    ok("state { n: int }\ntext @k n=(n)\nstrings en { k \"{n} left, {{braces}}\" }\n");
    ok("state { v: text }\ntext-input name=\"x\" bind=v on.change={ v = v }\n");
}
