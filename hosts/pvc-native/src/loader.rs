//! Artifact loading through `pvc_ir::load_signed`. A tampered / wrongly signed / non-canonical artifact is
//! refused here, before any window or GPU object exists. The runtime verifies again in `Runtime::from_signed`.
use std::path::Path;

pub struct Loaded {
    pub bytes: Vec<u8>,
    pub sha256: String,
}

/// Trusted publisher key. This session has one key: the dev-ir TEST key (public seed, see
/// `pvc_ir::sign::testkey`). A real deployment would pin a publisher key instead.
pub fn verify(bytes: &[u8]) -> Result<pvc_ir::Module, pvc_ir::LoadError> {
    pvc_ir::load_signed(bytes, &pvc_ir::sign::testkey::verifying_key())
}

pub fn load(path: &Path) -> Result<Loaded, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let module = verify(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let sha256 = pvc_ir::sign::digest_hex(&pvc_ir::canonicalize(&module));
    Ok(Loaded { bytes, sha256 })
}
