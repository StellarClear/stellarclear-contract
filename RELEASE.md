# SettlementRegistry Release Artifact Verification

This document specifies the verification chain and procedures for ensuring that published and deployed `SettlementRegistry` Soroban smart contracts match the exact source revision and tagged release.

---

## 1. Release Verification Chain

Every contract release is anchored through a strict, deterministic verification pipeline:

```text
Git Tag (e.g. v0.1.0)
       ↓
Source Revision (Git Commit SHA)
       ↓
Reproducible WASM Build (wasm32v1-none)
       ↓
SHA-256 Checksum (`settlement_registry.wasm.sha256`)
       ↓
Published Release Artifact (`artifacts/settlement_registry.wasm`)
       ↓
Deployed Contract (`SETTLEMENT_REGISTRY_CONTRACT_ID`)
```

---

## 2. Release Chain Verification Requirements

The release verification **must fail** if any of the following conditions occur:
1. **Artifact Mismatch**: The compiled WASM binary differs byte-for-byte from the published release artifact.
2. **Checksum Mismatch**: The SHA-256 hash computed from the freshly built WASM does not match `settlement_registry.wasm.sha256`.
3. **Version Mismatch**: The version declared in `Cargo.toml` differs from the release manifest version or tag.
4. **Deployed Metadata Mismatch**: The deployed contract bytecode or interface does not match the release candidate artifact.

---

## 3. Verification Commands

### Automated Release Artifact Verification

```bash
# Rebuilds from source revision and checks byte-for-byte reproducibility
./scripts/verify-artifact.sh
```

### Deployed Contract Verification

```bash
# Verifies deployed contract ID on testnet / mainnet against release artifact
./scripts/verify-deployment.sh <CONTRACT_ID> [NETWORK]
```

### Programmatic Integration Test Matrix

```bash
# Executes Soroban test environment release artifact checks
cargo test --test release_artifact -- --nocapture
```
