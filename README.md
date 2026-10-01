# StellarClear Contract (`stellarclear-contract`)

<p align="center">
  <img src="./docs/assets/banner.jpeg" alt="StellarClear Banner" width="100%" />
</p>

<p align="center">
  <a href="./LICENSE"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License"></a>
  <a href="https://github.com/StellarClear/stellarclear-contract/actions/workflows/ci.yml"><img src="https://github.com/StellarClear/stellarclear-contract/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/StellarClear/stellarclear-contract/actions/workflows/security-verification.yml"><img src="https://github.com/StellarClear/stellarclear-contract/actions/workflows/security-verification.yml/badge.svg" alt="Security Verification"></a>
  <a href="https://github.com/StellarClear/stellarclear-contract/actions/workflows/release-verification.yml"><img src="https://github.com/StellarClear/stellarclear-contract/actions/workflows/release-verification.yml/badge.svg" alt="Release Verification"></a>
  <a href="./SECURITY.md"><img src="https://img.shields.io/badge/Audit-Unaudited-orange.svg" alt="Audit Status"></a>
  <a href="https://stellar.org/soroban"><img src="https://img.shields.io/badge/Soroban-v27.0-7023e0.svg" alt="Soroban"></a>
</p>

StellarClear is an open-source, Stellar-native settlement evidence and reconciliation protocol. It deterministically compares expected settlement instructions with observed Stellar payments, records machine-readable reconciliation outcomes, and anchors verifiable cryptographic proof and attestations on Soroban smart contracts.

This repository contains the authoritative smart contract layer (`SettlementRegistry`). The companion off-chain SDK, REST API, ingestion indexer, and reconciliation engine live in [`StellarClear/stellarclear-app`](https://github.com/StellarClear/stellarclear-app).

---

## Protocol Architecture & Design

`SettlementRegistry` operates as an on-chain state machine and cryptographic evidence anchor. It records immutable commitments to trade settlement terms, on-chain observations, reconciliation breaks, two-party dispute resolutions, and multi-party cryptographic attestations.

### Architecture Boundary

```text
┌─────────────────────────────────────────────────────────┐
│                 Off-Chain Layer (API/SDK)               │
│ - Private settlement documents & canonicalization       │
│ - SHA-256 commitment generation (terms, obs, res)       │
│ - Stellar ledger indexing & automated break detection   │
└──────────────────────────┬──────────────────────────────┘
                           │ Anchors State & Proofs
                           ▼
┌─────────────────────────────────────────────────────────┐
│           Soroban Smart Contract (This Repo)            │
│               SettlementRegistry Contract               │
│ - Case Lifecycle (Open -> Observed -> Matched/Break...) │
│ - Observer Authorization & Evidence Records             │
│ - Dispute Opening & Two-Party Resolution Agreement      │
│ - Cryptographic Attestations & Finalization             │
└─────────────────────────────────────────────────────────┘
```

### On-Chain Responsibilities

The contract is authoritative for:
- **Settlement Lifecycle**: Strict state machine enforcement across all case stages.
- **Commitment Anchoring**: Deterministic 32-byte SHA-256 hashes for terms, observations, and dispute resolutions.
- **Observer Registry**: Whitelisted observer management (`add_observer`, `remove_observer`) authorized by the contract administrator.
- **Reconciliation Decisions**: Categorized decision records (`Matched` or `Break` with standardized `BreakCode`).
- **Two-Party Disputes**: Permissionless dispute submission and dual-party resolution agreements.
- **Cryptographic Attestations**: Multi-party attestations (`Owner`, `Counterparty`, `Observer`).
- **Irreversible Finalization**: Immutable locking of verified settlement cases.
- **Typed Protocol Events**: Comprehensive Soroban event logging for downstream indexers.

### Deliberately Out of Scope

The smart contract deliberately **does NOT**:
- Store unencrypted, raw financial or private counterparty trade details on-chain.
- Execute direct token transfers, custodial escrows, or liquidity movements.
- Perform floating-point or non-deterministic mathematical operations.
- Contain administrative backdoors or emergency state mutation bypasses (contract administration is strictly limited to managing observer registration and cannot alter case data, override decisions, or bypass state machine transitions).

---

## State Machine

Settlement cases follow an explicit, deterministic state progression:

```text
Open ──► Observed ──┬──► Matched ──────────► Finalized
                    │
                    └──► Break ──► Disputed ──► Resolved ──► Finalized
```

---

## Quick Start & Compilation

### Prerequisites
- **Rust**: `stable` (`1.84.0+` MSRV)
- **Wasm Target**: `wasm32v1-none`
- **Stellar CLI**: `28.1.0+`
- **Soroban SDK**: `27.0.4+`

### Practical Quick-Start Commands

```bash
# 1. Clone the repository
git clone https://github.com/StellarClear/stellarclear-contract.git
cd stellarclear-contract

# 2. Add the WebAssembly compilation target
rustup target add wasm32v1-none

# 3. Run all unit, security regression, and invariant tests
cargo test --workspace

# 4. Check formatting and clippy linter
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings

# 5. Compile the optimized Soroban Wasm contract binary
stellar contract build
```

The optimized contract artifact is produced at:
```text
target/wasm32v1-none/release/settlement_registry.wasm
```

To verify the SHA-256 artifact checksum:
```bash
sha256sum target/wasm32v1-none/release/settlement_registry.wasm
```

---

## Deployment & Release Documentation

- **Release Verification & Manifests**: Canonical release procedures and metadata manifests are documented in [`RELEASE.md`](./RELEASE.md).
- **Deployment & Testnet Guide**: Network setup, contract deployment, parameter initialization, and testnet lifecycle verification are detailed in [`DEPLOYMENT.md`](./DEPLOYMENT.md).
- **Published Artifacts**: Versioned WebAssembly binaries, hashes, and release manifests are in [`artifacts/`](./artifacts/README.md).
- **Security Policy & Invariants**: Authorization rules, invariants, and reporting guidelines are in [`SECURITY.md`](./SECURITY.md).

---

## Maintainers

| Name | GitHub | Role |
| :--- | :--- | :--- |
| **Adejumo** | [@Adejumo-2](https://github.com/Adejumo-2) | Lead Developer & Maintainer |
| **Smog** | [@smog123](https://github.com/smog123) | Core Contributor & Maintainer |

---

## Community & Support

- **GitHub Discussions**: [Ask questions and share ideas](https://github.com/StellarClear/stellarclear-contract/discussions)
- **GitHub Issues**: [Report bugs or suggest features](https://github.com/StellarClear/stellarclear-contract/issues)
- **Protocol Monorepo**: [StellarClear SDK, API & Indexer](https://github.com/StellarClear/stellarclear-app)

---

## Contributing

We welcome contributions from the community! Please read our [`CONTRIBUTING.md`](./CONTRIBUTING.md) guide and [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md) before submitting pull requests.

---

## Contributors

Thanks to all the contributors who help build and improve the StellarClear protocol!

<p align="center">
  <a href="https://github.com/StellarClear/stellarclear-contract/graphs/contributors">
    <img src="https://contrib.rocks/image?repo=StellarClear/stellarclear-contract" alt="StellarClear Contract Contributors" />
  </a>
</p>

---

## License

Licensed under the Apache License, Version 2.0. See [`LICENSE`](./LICENSE) for details.
