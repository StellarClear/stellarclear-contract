# Changelog

All notable changes to the `StellarClear` Soroban smart contract repository (`stellarclear-contract`) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] - 2026-09-30

### Added
- **SettlementRegistry Soroban Contract**:
  - Authoritative on-chain state machine (`Open` $\rightarrow$ `Observed` $\rightarrow$ `Matched`/`Break` $\rightarrow$ `Disputed` $\rightarrow$ `Resolved` $\rightarrow$ `Finalized`).
  - Terms commitment anchoring (`BytesN<32>`) and on-chain observation recording with transaction hash and ledger index.
  - Granular reconciliation breaks with standardized typed `BreakCode` enum (`AmountMismatch`, `DestinationMismatch`, `AssetMismatch`, `MemoMismatch`, `MissingTransaction`, `UnexpectedTransaction`, `TimingViolation`, `Other`).
  - Two-party dispute resolution agreement protocol (`open_dispute`, `submit_resolution`).
  - Cryptographic multi-party attestations (`Owner`, `Counterparty`, `Observer`) via `submit_attestation`.
  - Terminal-state immutability and permanent state locking on finalized settlement cases.
  - Comprehensive contract events emitted for 100% of state-mutating actions.
- **Security & Quality Regression Suites**:
  - Full authorization boundaries and adversarial test matrix.
  - Malformed-input and resource-boundary scaling coverage.
  - Terminal-state immutability and duplicate call prevention tests.
  - 102 automated tests passing with zero warnings.
- **Reproducible Release Artifact Packaging & Verification**:
  - Deterministic WebAssembly compilation (`wasm32v1-none`) with SHA-256 checksum `1018a81b1ac95046cb00466ceda7ee347204c08b71b1c51b3c9611dd32215d66`.
  - Machine-readable release provenance manifest (`artifacts/release-manifest.json` and `artifacts/v0.1.0/`).
  - Release chain integration verification test suite (`tests/release_artifact.rs`).
  - Programmatic deployed testnet verification matrix (`tests/deployed_testnet.rs`).
  - Shell automation scripts for building (`scripts/build.sh`), checking (`scripts/check.sh`), packaging (`scripts/release.sh`), artifact verification (`scripts/verify-artifact.sh`), and testnet deployment verification (`scripts/verify-deployment.sh`, `scripts/testnet-verification.sh`).
- **CI / CD Infrastructure**:
  - GitHub Actions workflows for continuous integration (`.github/workflows/ci.yml`), release verification (`.github/workflows/release-verification.yml`), and security regression testing (`.github/workflows/security-verification.yml`).
  - Automated system dependency provisioning (`dbus`, `libdbus-1-dev`, `pkg-config`, `libssl-dev`, `libudev-dev`).
- **Documentation**:
  - Security model, authorization matrix, threat model, and audit status in `SECURITY.md`.
  - Production/testnet release checklist, verification chain, and packaging specification in `RELEASE.md`.
  - Step-by-step deployment guide and testnet verification instructions in `DEPLOYMENT.md`.
  - Drips protocol funding readiness in `FUNDING.json`.
