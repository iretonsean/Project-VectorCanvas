//! PVC-IR v0 (Session 2 subset): terse parser, canonical form, validator, Ed25519 signing.
//!
//! Hard wall (G4 P1/P2): no network access, no language-model call, no clock, no randomness in this crate.
//!
//! ```
//! use pvc_ir::*;
//! let src = r#"pvc 0.1 use std@0.1 theme dark
//! button "Go" on.press=ping
//! cap net.main hosts=["h.invalid"] methods=["POST"]
//! effect ping uses net.main
//! "#;
//! let out = compile_terse(src).unwrap();
//! let bytes = sign(&out.module, &sign::testkey::signing_key());
//! let m = load_signed(&bytes, &sign::testkey::verifying_key()).unwrap();
//! assert_eq!(canonicalize(&m), canonicalize(&out.module));
//! ```

pub mod ast;
pub mod canon;
pub mod diag;
pub mod eval;
pub mod expand;
pub mod lex;
pub mod parse;
pub mod pred;
pub mod sign;
pub mod theme;
pub mod validate;

pub use ast::*;
pub use canon::{canonicalize, from_canonical, CanonError};
pub use diag::{Diagnostic, Edit, Fix, Location, Severity, Span};
pub use sign::{load_signed, sign, LoadError};
pub use validate::validate;

/// Result of a successful compile: the canonical module plus non-blocking diagnostics.
#[derive(Clone, Debug)]
pub struct Compiled {
    pub module: Module,
    /// Warnings and infos (no errors: errors make `compile_terse` return `Err`).
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse terse text and expand it to the canonical AST. Raises `PVC-E-PARSE`, `PVC-E-SCHEMA`,
/// `PVC-E-UNKNOWN-KIND`, `PVC-E-UNKNOWN-TOKEN` (and a few schema-level `ID-COLLISION`/`TYPE`).
/// Does **not** run reference-level validation; see [`compile_terse`].
pub fn parse_terse(src: &str) -> Result<Module, Vec<Diagnostic>> {
    let raw = parse::parse_raw(src).map_err(|d| vec![d])?;
    expand::expand(&raw)
}

/// `parse_terse` + `validate`. `Err` carries every diagnostic (errors first in the validator's order).
pub fn compile_terse(src: &str) -> Result<Compiled, Vec<Diagnostic>> {
    let module = parse_terse(src)?;
    let diagnostics = validate(&module);
    if validate::has_errors(&diagnostics) {
        return Err(diagnostics);
    }
    Ok(Compiled { module, diagnostics })
}
