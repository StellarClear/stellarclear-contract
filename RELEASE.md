# SettlementRegistry Release Process & Artifact Specification

This document details the repeatable release process, artifact packaging, verification, and distribution for the `SettlementRegistry` Soroban smart contract.

---

## 1. Release Artifact Package

Every contract release produces a canonical set of verifiable artifacts:

| Artifact | Location | Purpose |
| :--- | :--- | :--- |
| **WASM Binary** | `artifacts/settlement_registry.wasm` | Bytecode artifact deployed to Soroban networks |
| **Checksum File** | `artifacts/settlement_registry.wasm.sha256` | SHA-256 hash for byte-for-byte integrity checks |
| **Release Manifest** | `artifacts/release-manifest.json` | Comprehensive machine-readable build metadata |
| **Versioned Archive** | `artifacts/v<VERSION>/` | Immutable per-version archive of binary, checksum, and manifest |

---

## 2. Generating Release Artifacts

To create the release package from the repository source:

```bash
# 1. Run all quality and test checks
./scripts/check.sh

# 2. Package release artifacts
./scripts/release.sh
```

---

## 3. Release Manifest Specification

The release manifest records:
- **Contract Name & Version**: Aligned with workspace `Cargo.toml`.
- **Source Revision**: Exact Git commit SHA and branch name.
- **WASM Metadata**: Canonical file name, SHA-256 checksum, and file size in bytes.
- **Toolchain Environment**: `rustc` compiler version, `stellar-cli` version, and `soroban-sdk` version.
- **Target Network Metadata**: Network passphrases, RPC endpoints, and deployed contract addresses (when deployed).

---

## 4. Verification

Re-verify the built artifact against the published checksum:

```bash
sha256sum -c artifacts/settlement_registry.wasm.sha256
```
