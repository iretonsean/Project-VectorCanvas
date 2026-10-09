# RFC note: `sha2 =0.10.9` (pvc-ir)

**What:** SHA-256 (RustCrypto). **Version:** exactly `=0.10.9` (same version `ed25519-dalek 2.2.0` already depends on, so no second copy). **Pre-approved** by D-10.
**Why nothing pinned covers it:** G4 §3/§11 want content hashes (module digest, font-set hashes in the manifest) and the test keypair seed is derived as `sha256(phrase)`; the G7 table has no hash.
**What it pulls in:** `digest`, `cfg-if`, `cpufeatures` (all already in the ed25519-dalek subtree). Default features.
**Hard-wall check:** no network/LLM crate in either target's `cargo tree -e normal` (`evidence/hardwall-scan.txt`); pure computation. wasm32 result identical to native (`evidence/d1-wasm.txt`, in-wasm `sha256_hex`).
