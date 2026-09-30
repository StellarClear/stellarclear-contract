# SettlementRegistry Release Process & Security Review Checklist

This document details the complete contract release, security review, reproducible artifact packaging, and deployment verification procedures for the `SettlementRegistry` Soroban smart contract.

---

## ⚠️ Security Status & Audit Disclaimer

> [!CAUTION]
> **UNAUDITED PROTOTYPE / DEVELOPMENT IMPLEMENTATION**:
> The `SettlementRegistry` smart contract is under active development and has **not** undergone an independent third-party security audit or formal verification.
> Do **NOT** deploy or use this contract in production environments handling real financial value without prior independent security auditing.

---

## 📦 StellarClear Contract v0.1.0 Release Overview

### Release Summary

| Property | Value |
| :--- | :--- |
| **Contract Name** | `settlement_registry` |
| **Release Version** | `0.1.0` |
| **Release Tag** | `v0.1.0` |
| **WASM Artifact** | [`artifacts/settlement_registry.wasm`](./artifacts/settlement_registry.wasm) |
| **SHA-256 Checksum** | `1018a81b1ac95046cb00466ceda7ee347204c08b71b1c51b3c9611dd32215d66` |
| **WASM Size** | `22,722 bytes` (optimized) |
| **Compilation Target** | `wasm32v1-none` |
| **Soroban SDK** | `27.0.4` |
| **Stellar CLI** | `28.1.0` |
| **License** | Apache-2.0 |

### Exported Smart Contract Functions (15)

1. `__constructor(admin: Address)`: Initializes contract admin during deployment.
2. `add_observer(observer: Address)`: Admin registers authorized observer address.
3. `remove_observer(observer: Address)`: Admin revokes observer authorization.
4. `is_observer(observer: Address) -> bool`: Checks whether an address is an authorized observer.
5. `create_case(case_id, owner, counterparty, terms_commitment, expires_at_ledger)`: Opens new settlement case.
6. `record_observation(observer, case_id, tx_hash, observed_ledger, observation_commitment)`: Anchors Stellar transaction observation.
7. `record_match(observer, case_id)`: Records positive reconciliation match.
8. `record_break(observer, case_id, break_code)`: Records reconciliation break with standardized `BreakCode`.
9. `open_dispute(disputant, case_id, dispute_commitment)`: Counterparty or owner opens formal dispute on broken case.
10. `submit_resolution(party, case_id, resolution_commitment)`: Records agreed dispute resolution terms.
11. `submit_attestation(case_id, role, attestation_commitment)`: Submits cryptographic attestation (Owner, Counterparty, Observer).
12. `finalize_case(case_id)`: Irreversibly locks settlement outcome and finalizes case.
13. `get_case(case_id) -> Case`: Retrieves complete on-chain case record.
14. `get_attestation(case_id, party) -> Option<Attestation>`: Queries registered attestation for a specific party.
15. `get_resolution(case_id, party) -> Option<BytesN<32>>`: Queries registered dispute resolution terms for a specific party.

---

## 1. Security Review & Release-Candidate Checklist

Before tagging or publishing any release, maintainers and deployers must complete and verify every step of this checklist:

### Pre-Release Quality & Environment Checks

- [ ] **Clean Working Tree**: Ensure `git status` reports a clean working tree with no uncommitted or untracked changes.
- [ ] **Tests Pass**: Full workspace unit, invariant, adversarial, boundary, and fuzz test suites pass (`cargo test --workspace`).
- [ ] **Formatting Passes**: Source code conforms to Rust formatting standards (`cargo fmt --all -- --check`).
- [ ] **Clippy Passes**: Linter passes cleanly with zero warnings (`cargo clippy --workspace --all-targets --all-features -- -D warnings`).
- [ ] **WASM Builds**: Deterministic WebAssembly binary compiles successfully with `wasm32v1-none` target (`./scripts/build.sh`).
- [ ] **Checksum Recorded**: SHA-256 hash generated and recorded in `artifacts/settlement_registry.wasm.sha256` and release manifest.
- [ ] **Release Version Recorded**: Workspace version in `Cargo.toml` matches tag and release notes (`0.1.0`).

### Deployment & Post-Deployment Verification

- [ ] **Deployment Network Recorded**: Explicit target network parameters configured (Testnet / Mainnet RPC URL, network passphrase).
- [ ] **Contract ID Recorded**: Deployed Soroban contract address (`CA...` or `CB...`) captured and exported in environment (`SETTLEMENT_REGISTRY_CONTRACT_ID`).
- [ ] **Deployed Artifact Verified**: Deployed contract instance verified on target network using automated verification suite (`./scripts/testnet-verification.sh` & `./scripts/verify-deployment.sh`).
- [ ] **Artifact Integrity Verified**: Published WASM artifact byte-for-byte matches tagged source build (`./scripts/verify-artifact.sh` & `cargo test --test release_artifact`).
- [ ] **Security Assumptions Reviewed**: Review role separation, observer trust model, lack of backdoors, and state immutability in [`SECURITY.md`](./SECURITY.md).
- [ ] **Upgrade Policy Reviewed**: Review contract instance versioning and historical state retention policies.
- [ ] **Prototype / Audit Disclaimer Preserved**: Ensure security disclaimers are preserved in all distribution artifacts.

---

## 2. Release Artifact Package

Every contract release produces a canonical set of verifiable artifacts:

| Artifact | Location | Purpose |
| :--- | :--- | :--- |
| **WASM Binary** | `artifacts/settlement_registry.wasm` | Optimized byte-for-byte binary for deployment |
| **Checksum File** | `artifacts/settlement_registry.wasm.sha256` | SHA-256 cryptographic verification checksum |
| **Release Manifest** | `artifacts/release-manifest.json` | Machine-readable build, compiler, and network metadata |
| **Versioned Archive** | `artifacts/v<VERSION>/` | Immutable per-version snapshot of binary, checksum, and manifest |

---

## 3. Step-by-Step Release Procedure

### Step 1: Execute Complete Quality Verification

```bash
# Runs fmt check, clippy with -D warnings, unit/integration tests, and WASM build
./scripts/check.sh
```

### Step 2: Package Release Artifacts

```bash
# Generates canonical WASM, SHA-256 checksum, manifest, and versioned archive
./scripts/release.sh
```

### Step 3: Verify Release Artifact Integrity

```bash
# Rebuilds from source and verifies byte-for-byte reproducibility against manifest
./scripts/verify-artifact.sh
cargo test --test release_artifact -- --nocapture
```

### Step 4: Verify Deployment on Testnet

```bash
# Executes automated deployment and full lifecycle test suite on Soroban testnet
./scripts/testnet-verification.sh
cargo test --test deployed_testnet -- --nocapture
```

### Step 5: Verify Live Deployed Contract Instance

```bash
./scripts/verify-deployment.sh <DEPLOYED_CONTRACT_ID> testnet
```

---

## 4. Release Manifest Specification

The `release-manifest.json` provides comprehensive machine-readable provenance:

```json
{
  "contract_name": "settlement_registry",
  "version": "0.1.0",
  "git_commit": "81d7a0eac4a54fb0a13d4e2515f7a7a1217fe2f9",
  "git_branch": "feat/settlement-registry-protocol",
  "timestamp": "2026-09-30T07:48:30Z",
  "wasm_file": "settlement_registry.wasm",
  "wasm_sha256": "1018a81b1ac95046cb00466ceda7ee347204c08b71b1c51b3c9611dd32215d66",
  "wasm_size_bytes": 22722,
  "rustc_version": "rustc 1.84.0",
  "stellar_cli_version": "stellar 28.1.0",
  "soroban_sdk_version": "27.0.4",
  "target": "wasm32v1-none",
  "networks": {
    "testnet": {
      "rpc_url": "https://soroban-testnet.stellar.org",
      "network_passphrase": "Test SDF Network ; September 2015",
      "contract_id": ""
    },
    "mainnet": {
      "rpc_url": "https://mainnet.sorobanrpc.com",
      "network_passphrase": "Public Global Stellar Network ; July 2015",
      "contract_id": ""
    }
  }
}
```

---

## 5. Security & Upgrade Policies

1. **Protocol Immutability**: All finalized cases, commitments, and reconciliation decisions are permanently immutable on-chain.
2. **Contract Instance Versioning**: Upgrades occur through deploying new versioned contract instances (`v0.1.0`, `v0.2.0`, etc.). Prior instances remain permanently queryable.
3. **No Administrative Backdoors**: Admin privileges are strictly restricted to observer registration (`add_observer`, `remove_observer`). There are no emergency overrides or asset transfer capabilities.
