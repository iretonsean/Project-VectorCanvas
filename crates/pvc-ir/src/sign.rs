//! Ed25519 signing and verify-on-load (G4 §10 rule 4, §11).
//!
//! Signed artifact layout (ASCII header, then the canonical bytes verbatim):
//! ```text
//! PVC-SIGNED 1 ed25519 <publisher public key, 64 hex>\n
//! <signature, 128 hex>\n
//! <canonical module bytes>
//! ```
//! The signature covers `DOMAIN || canonical bytes`. The embedded public key only *identifies* the
//! signer; the loader verifies against the key the caller trusts and refuses a different one.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::ast::Module;
use crate::canon::{canonicalize, from_canonical};

pub const DOMAIN: &[u8] = b"PVC-IR-v0.1-artifact\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// Header line missing or malformed.
    BadFrame(String),
    /// The artifact names a different publisher key than the one the caller trusts.
    KeyMismatch,
    /// Signature does not verify over the canonical bytes (includes any post-signing tampering).
    BadSignature,
    /// Signature verified but the body is not a valid canonical module.
    Canonical(String),
    /// Body parses but re-serialises to different bytes (not canonical).
    NotCanonical,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::BadFrame(m) => write!(f, "bad artifact frame: {}", m),
            LoadError::KeyMismatch => write!(f, "artifact is signed by a different key than the trusted key"),
            LoadError::BadSignature => write!(f, "signature verification failed: refusing to load"),
            LoadError::Canonical(m) => write!(f, "invalid canonical module: {}", m),
            LoadError::NotCanonical => write!(f, "module bytes are not in canonical form"),
        }
    }
}
impl std::error::Error for LoadError {}

pub fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

pub fn from_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len() / 2).map(|i| u8::from_str_radix(s.get(2 * i..2 * i + 2)?, 16).ok()).collect()
}

/// sha256 of the canonical bytes, lowercase hex (the module digest).
pub fn digest_hex(canonical: &[u8]) -> String {
    to_hex(&Sha256::digest(canonical))
}

/// Sign already-canonical bytes. Exposed so tests can build artifacts without a `Module`.
pub fn sign_canonical(canonical: &[u8], key: &SigningKey) -> Vec<u8> {
    let mut msg = Vec::with_capacity(DOMAIN.len() + canonical.len());
    msg.extend_from_slice(DOMAIN);
    msg.extend_from_slice(canonical);
    let sig = key.sign(&msg);
    let mut out = format!(
        "PVC-SIGNED 1 ed25519 {}\n{}\n",
        to_hex(key.verifying_key().as_bytes()),
        to_hex(&sig.to_bytes())
    )
    .into_bytes();
    out.extend_from_slice(canonical);
    out
}

/// Canonicalise and sign a module.
pub fn sign(m: &Module, key: &SigningKey) -> Vec<u8> {
    sign_canonical(&canonicalize(m), key)
}

/// Verify a signed artifact against the trusted key and parse it. Refuses a bad signature.
pub fn load_signed(bytes: &[u8], trusted: &VerifyingKey) -> Result<Module, LoadError> {
    let (l1, rest) = split_line(bytes).ok_or_else(|| LoadError::BadFrame("no header line".into()))?;
    let (l2, body) = split_line(rest).ok_or_else(|| LoadError::BadFrame("no signature line".into()))?;
    let l1 = std::str::from_utf8(l1).map_err(|_| LoadError::BadFrame("header not ascii".into()))?;
    let l2 = std::str::from_utf8(l2).map_err(|_| LoadError::BadFrame("signature not ascii".into()))?;
    let parts: Vec<&str> = l1.split(' ').collect();
    if parts.len() != 4 || parts[0] != "PVC-SIGNED" || parts[1] != "1" || parts[2] != "ed25519" {
        return Err(LoadError::BadFrame("unsupported header".into()));
    }
    let pk = from_hex(parts[3]).ok_or_else(|| LoadError::BadFrame("bad key hex".into()))?;
    if pk.as_slice() != trusted.as_bytes() {
        return Err(LoadError::KeyMismatch);
    }
    let sig_bytes = from_hex(l2).ok_or_else(|| LoadError::BadFrame("bad signature hex".into()))?;
    let sig_arr: [u8; 64] = sig_bytes.try_into().map_err(|_| LoadError::BadFrame("signature length".into()))?;
    let sig = Signature::from_bytes(&sig_arr);
    let mut msg = Vec::with_capacity(DOMAIN.len() + body.len());
    msg.extend_from_slice(DOMAIN);
    msg.extend_from_slice(body);
    trusted.verify_strict(&msg, &sig).map_err(|_| LoadError::BadSignature)?;
    let m = from_canonical(body).map_err(|e| LoadError::Canonical(e.0))?;
    if canonicalize(&m) != body {
        return Err(LoadError::NotCanonical);
    }
    Ok(m)
}

fn split_line(b: &[u8]) -> Option<(&[u8], &[u8])> {
    let i = b.iter().position(|c| *c == b'\n')?;
    Some((&b[..i], &b[i + 1..]))
}

/// A fixed **TEST** keypair, committed on purpose so artifacts and tests are reproducible.
/// The seed is public: it must never sign anything real. The runtime in this session trusts it
/// only because the session's specimens are signed with it.
pub mod testkey {
    use super::*;

    pub const NOTICE: &str = "PVC session-2 TEST key. Seed is public. Not for production use.";
    pub const SEED_PHRASE: &str = "pvc-session2-test-key-DO-NOT-USE-IN-PRODUCTION";

    pub fn signing_key() -> SigningKey {
        let seed: [u8; 32] = Sha256::digest(SEED_PHRASE.as_bytes()).into();
        SigningKey::from_bytes(&seed)
    }
    pub fn verifying_key() -> VerifyingKey {
        signing_key().verifying_key()
    }
    pub fn public_hex() -> String {
        to_hex(verifying_key().as_bytes())
    }
}
