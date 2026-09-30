# SettlementRegistry Release Process & Security Review Checklist

This document details the complete contract release, security review, and deployment verification procedures for the `SettlementRegistry` Soroban smart contract.

---

## ⚠️ Security Status & Audit Disclaimer

> [!CAUTION]
> **UNAUDITED PROTOTYPE / DEVELOPMENT IMPLEMENTATION**:
> The `SettlementRegistry` smart contract is under active development and has **not** undergone an independent third-party security audit or formal verification.
> Do **NOT** deploy or use this contract in production environments handling real financial value without prior independent security auditing.

---

## 1. Security Review & Release-Candidate Checklist

Before tagging or publishing any release, maintainers and deployers must complete and verify every item of this checklist:

### Section A: Pre-Release Quality & Environment Checks

- [ ] **Clean Working Tree**: Ensure `git status` reports a clean working tree with no uncommitted or untracked changes.
- [ ] **Formatting Passes**: Source code conforms to Rust formatting standards:
  ```bash
  cargo fmt --all -- --check
  ```
- [ ] **Clippy Passes**: Linter passes cleanly with zero warnings:
  ```bash
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```
- [ ] **Test Suite Passes**: Full workspace unit, invariant, boundary, fuzz, and integration tests pass cleanly:
  ```bash
  cargo test --workspace
  ```
- [ ] **Deterministic WASM Build**: WebAssembly binary compiles successfully with `wasm32v1-none` target:
  ```bash
  ./scripts/build.sh
  ```
- [ ] **Checksum Recorded**: SHA-256 hash generated and recorded in `target/wasm32v1-none/release/settlement_registry.wasm.sha256`.
- [ ] **Release Version Recorded**: Workspace version in `Cargo.toml` matches tag and release notes (`0.1.0`).

### Section B: Release Artifact Integrity & Deployment Verification

- [ ] **Artifact Chain Verified**: Rebuilds from tagged source revision and verifies byte-for-byte reproducibility:
  ```bash
  ./scripts/verify-artifact.sh
  cargo test --test release_artifact -- --nocapture
  ```
- [ ] **Deployment Verification**: Verify deployed contract interface and bytecode on target network:
  ```bash
  ./scripts/verify-deployment.sh <CONTRACT_ID> [testnet|mainnet]
  ```
- [ ] **Deployment Parameters Recorded**: Target network passphrase, RPC URL, admin address, and contract ID recorded.

### Section C: Security Review & Invariant Confirmation

- [ ] **Authorization Invariants**: Review role permissions in [`SECURITY.md`](./SECURITY.md) (Owner, Counterparty, Admin, Observer).
- [ ] **State Machine Invariants**: Verify terminal state immutability on finalized cases.
- [ ] **Event Integrity**: Ensure 100% of state-mutating actions emit typed Soroban contract events.
- [ ] **Known Limitations Reviewed**: Non-custodial scope, off-chain matching reliance, and observer authorization acknowledged.
- [ ] **Upgrade Policy Reviewed**: New contract instance versioning and historical state retention policies confirmed.
- [ ] **Prototype / Audit Disclaimer Preserved**: Security disclaimers preserved across all public documentation.

---

## 2. Release Verification Chain

Every contract release is verified through the canonical pipeline:

```text
Git Tag (e.g. v0.1.0)
       ↓
Source Revision (Git Commit SHA)
       ↓
Reproducible WASM Build (wasm32v1-none)
       ↓
SHA-256 Checksum (`settlement_registry.wasm.sha256`)
       ↓
Published Release Artifact
       ↓
Deployed Contract (`SETTLEMENT_REGISTRY_CONTRACT_ID`)
```

---

## 3. Step-by-Step Release Procedure

```bash
# 1. Run all quality, security, and release-candidate checks
./scripts/check.sh

# 2. Verify artifact integrity against source revision
./scripts/verify-artifact.sh

# 3. Verify deployed contract instance
./scripts/verify-deployment.sh <CONTRACT_ID> testnet
```
