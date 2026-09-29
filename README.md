# StellarClear Contract (`stellarclear-contract`)

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](./LICENSE)
[![CI](https://github.com/StellarClear/stellarclear-contract/actions/workflows/ci.yml/badge.svg)](./.github/workflows/ci.yml)

StellarClear is an open-source, Stellar-native settlement evidence and reconciliation protocol that compares expected settlement instructions with observed Stellar settlement, records machine-readable reconciliation outcomes, and anchors verifiable evidence and attestations on Soroban.

This repository contains the smart contract layer (`SettlementRegistry`) for the protocol. Off-chain SDK/API/indexer/matcher live in [`StellarClear/stellarclear`](https://github.com/StellarClear/stellarclear).

---

## Protocol Overview

`SettlementRegistry` serves as an on-chain state machine and cryptographic evidence anchor. It records immutable commitments to settlement terms, on-chain observations, reconciliation breaks, two-party dispute resolutions, and multi-party cryptographic attestations.

### Architecture Boundary

```text
┌─────────────────────────────────────────────────────────┐
│                 Off-Chain Layer (API/SDK)               │
│ - Private settlement documents & canonicalization       │
│ - SHA-256 commitment generation                         │
│ - Ledger indexing & break detection                     │
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
- Settlement case lifecycle transitions
- Terms commitment anchoring (`BytesN<32>`)
- Observed transaction references & observation commitments
- Observer registry management
- Reconciliation decision records (`Matched` or `Break` with standardized `BreakCode`)
- Two-party dispute resolution commitments
- Cryptographic attestations (Owner, Counterparty, Observer)
- Irreversible case finalization
- Typed protocol events

### Deliberately Out of Scope

The contract deliberately **does NOT**:
- Store or process raw, unencrypted, or private financial records
- Execute token transfers, payments, or escrow locks
- Perform off-chain document matching or OCR parsing
- Implement floating-point or currency math
- Include admin backdoors or emergency state mutation overrides

---

## State Machine

Settlement cases follow an explicit, enforced state machine:

```text
Open ──► Observed ──┬──► Matched ──────────► Finalized
                    │
                    └──► Break ──► Disputed ──► Resolved ──► Finalized
```

---

## Development & Toolchain Setup

### Prerequisites

- **Rust**: `1.84.0+`
- **Target**: `wasm32v1-none`
- **Stellar CLI**: `28.1.0+`
- **Soroban SDK**: `27.0.4+`

### Installation

```bash
# Add the WebAssembly compilation target
rustup target add wasm32v1-none

# Install Stellar CLI
cargo install --locked stellar-cli --version 28.1.0
```

### Running Tests

```bash
cargo test --workspace
```

### Checking Formatting

```bash
cargo fmt --all -- --check
```

### Building the Contract Wasm

```bash
stellar contract build
```

The optimized contract artifact will be generated at:
`target/wasm32v1-none/release/settlement_registry.wasm`

---

## Current Status & Disclaimer

> [!WARNING]
> **UNAUDITED SOFTWARE**: This smart contract is under active development and has not undergone formal verification or an independent security audit. Do not use this contract in production environments handling real financial value without prior audit.

---

## Funding (Drips)

This repo is claimable on [Drips](https://www.drips.network). Ownership is proven via `FUNDING.json` on the default branch (`main`).

> `FUNDING.json` currently contains a placeholder `0x0000...` address. Maintainers: replace it with the project owner address during the Drips claim flow.

## Contributing

See [`CONTRIBUTING.md`](./CONTRIBUTING.md).

## License

Apache-2.0 — see [`LICENSE`](./LICENSE).
