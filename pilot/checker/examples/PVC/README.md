# PVC pass/fail examples — PENDING

Per brief §4: one hand-written passing and one failing `.pvc` per specimen (15 × 2) are validated against `check-pvc` once (a) dev-ir's v0 grammar covers the pilot node kinds (D-2) and (b) dev-render ships `pvc-probe` (`PVC_CHECKER_NEEDS.md`). Until then `checker_validation.md` marks the PVC column *pending*. The failing PVC examples will reuse the mutation names/expected item ids of `examples/mutations.mjs`.
`pvc/mock-probe.mjs` exercises the protocol client with baseline HTML; it is a wiring test only and is not PVC evidence.
