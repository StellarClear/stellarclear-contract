# StellarClear Contract (`stellarclear-contract`)

<p align="center">
  <img src="./docs/assets/banner.jpeg" alt="StellarClear Banner" width="100%" />
</p>

<p align="center">
  <a href="./LICENSE"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License"></a>
  <a href="https://github.com/StellarClear/stellarclear-contract/actions/workflows/ci.yml"><img src="https://github.com/StellarClear/stellarclear-contract/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/StellarClear/stellarclear-contract/actions/workflows/security-verification.yml"><img src="https://github.com/StellarClear/stellarclear-contract/actions/workflows/security-verification.yml/badge.svg" alt="Security Verification"></a>
  <a href="https://github.com/StellarClear/stellarclear-contract/actions/workflows/release-verification.yml"><img src="https://github.com/StellarClear/stellarclear-contract/actions/workflows/release-verification.yml/badge.svg" alt="Release Verification"></a>
  <a href="https://stellar.org/soroban"><img src="https://img.shields.io/badge/Soroban-v27.0-7023e0.svg" alt="Soroban"></a>
</p>

---

StellarClear is an open-source, Stellar-native settlement evidence and reconciliation protocol. It deterministically compares expected settlement instructions with observed Stellar payments, records machine-readable reconciliation outcomes, and anchors verifiable cryptographic proof and attestations on Soroban smart contracts.

This repository contains the authoritative smart contract layer (`SettlementRegistry`). The companion off-chain SDK, REST API, ingestion indexer, and reconciliation engine live in [`StellarClear/stellarclear-app`](https://github.com/StellarClear/stellarclear-app).

---

## Table of Contents

- [Protocol Architecture & Design](#protocol-architecture--design)
- [How This Fits with the App](#how-this-fits-with-the-app)
- [State Machine](#state-machine)
- [Public Interface](#public-interface)
- [Events](#events)
- [Break Codes](#break-codes)
- [Authorization & Trust Model](#authorization--trust-model)
- [Quick Start & Compilation](#quick-start--compilation)
- [Deployment & Release Documentation](#deployment--release-documentation)
- [Maintainers](#maintainers)
- [Community & Support](#community--support)
- [Contributing](#contributing)
- [Contributors](#contributors)
- [License](#license)

---

## Protocol Architecture & Design

`SettlementRegistry` operates as an on-chain state machine and cryptographic evidence anchor. It records immutable commitments to trade settlement terms, on-chain observations, reconciliation breaks, two-party dispute resolutions, and multi-party cryptographic attestations.

### Architecture Boundary

```text
┌─────────────────────────────────────────────────────────┐
│              Off-Chain App (stellarclear-app)           │
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

## How This Fits with the App

The off-chain companion repository [`StellarClear/stellarclear-app`](https://github.com/StellarClear/stellarclear-app) provides the REST API, ingestion indexers, reconciliation daemons, and client SDKs that interact directly with this smart contract.

### Worked Settlement Lifecycle Example

```text
  Off-Chain (API / SDK)                  On-Chain (SettlementRegistry)
─────────────────────────              ──────────────────────────────────
1. Alice & Bob agree on trade terms
   Compute terms SHA-256 hash ────────►  create_case() [Status: Open]

2. Bob executes payment on Stellar
   payment rails (Tx Hash: 0xabc...)

3. Observer daemon detects payment
   Matches payment against terms ─────►  record_observation() [Status: Observed]
                                  ─────►  record_match()       [Status: Matched]

4. Alice & Observer submit signatures ─► submit_attestation() (Owner + Observer)

5. Alice finalizes settlement ────────►  finalize_case()     [Status: Finalized (Terminal)]
```

#### Step-by-Step Flow:
1. **Trade Initiation**: Alice (`Owner`) and Bob (`Counterparty`) agree off-chain on settlement terms. Alice computes the canonical 32-byte SHA-256 hash `terms_commitment` and calls `create_case` on-chain, specifying an expiration ledger.
2. **Stellar Payment**: Bob executes a payment on Stellar containing the agreed reference memo.
3. **Observation & Reconciliation**: A registered StellarClear observer daemon ingests the ledger transaction, verifies that the asset, amount, destination, and memo match the terms, computes the `observation_commitment`, and calls `record_observation` followed by `record_match`. (If any discrepancy is found, it calls `record_break` with a typed `BreakCode`).
4. **Attestation Submission**: Alice and the assigned observer submit their cryptographic commitments via `submit_attestation`.
5. **Irreversible Finalization**: Alice invokes `finalize_case`. The contract verifies that all required attestations are registered, transitions the case to `Finalized`, and emits `CaseFinalized`. The case is now permanently read-only and immutable.

---

## State Machine

Settlement cases follow an explicit, deterministic state progression:

```text
Open ──► Observed ──┬──► Matched ─────────────────────────────────► Finalized
                    │
                    └──► Break ──► Disputed ──► Resolved ─────────► Finalized
```

### Supported Transitions:
- `Open -> Observed`: Triggered by `record_observation` from a registered observer.
- `Observed -> Matched`: Triggered by `record_match` from the assigned observer.
- `Observed -> Break`: Triggered by `record_break` with a `BreakCode` from the assigned observer.
- `Break -> Disputed`: Triggered by `open_dispute` from the case owner or counterparty.
- `Disputed -> Resolved`: Triggered by `submit_resolution` when both owner and counterparty independently submit matching 32-byte resolution commitments.
- `Matched -> Finalized`: Triggered by `finalize_case` from the owner when Owner and Observer attestations are present.
- `Resolved -> Finalized`: Triggered by `finalize_case` from the owner when Owner, Counterparty, and Observer attestations are present.

---

## Public Interface

The `SettlementRegistry` contract exposes 15 public functions:

| Function | Description | Callable By | Required State | Resulting State |
| :--- | :--- | :--- | :--- | :--- |
| `__constructor(admin: Address)` | One-time initialization setting the protocol admin and version `1`. | Deployer | Uninitialized | Initialized |
| `add_observer(observer: Address)` | Registers an authorized settlement observer address. | Contract Admin | Any (unregistered observer) | Observer Whitelisted |
| `remove_observer(observer: Address)` | Revokes authorization for an existing settlement observer. | Contract Admin | Any (registered observer) | Observer Revoked |
| `create_case(case_id, owner, counterparty, terms_commitment, expires_at_ledger)` | Opens a new settlement case with terms hash and expiration ledger. | Case Owner (`owner.require_auth()`) | Non-existent `case_id` | `Open` |
| `record_observation(observer, case_id, tx_hash, observed_ledger, observation_commitment)` | Records Stellar transaction hash and observation evidence for an open case. | Registered Observer (`require_observer_auth()`) | `Open` | `Observed` |
| `record_match(observer, case_id)` | Records a positive reconciliation decision for an observed case. | Registered Observer (`require_observer_auth()`) | `Observed` | `Matched` |
| `record_break(observer, case_id, break_code)` | Records a reconciliation break decision with standardized `BreakCode`. | Registered Observer (`require_observer_auth()`) | `Observed` | `Break` |
| `submit_attestation(case_id, role, commitment)` | Submits a 32-byte cryptographic attestation for `Owner`, `Counterparty`, or `Observer`. | Authorized Principal for Role | Any active case prior to `Finalized` | Attestation Recorded |
| `open_dispute(initiator, case_id, dispute_commitment)` | Opens a dispute against a case in `Break` state. | Case Owner or Counterparty | `Break` | `Disputed` |
| `submit_resolution(resolver, case_id, resolution_commitment)` | Submits resolution terms; transitions to `Resolved` once both parties agree. | Case Owner or Counterparty | `Disputed` | `Disputed` (1st party) / `Resolved` (matching 2nd party) |
| `finalize_case(case_id)` | Irreversibly locks settlement outcome after verifying required role attestations. | Case Owner (`owner.require_auth()`) | `Matched` (Owner+Observer) or `Resolved` (Owner+CP+Observer) | `Finalized` (Terminal) |
| `get_case(case_id) -> SettlementCase` | Queries complete on-chain case record (terms, status, observation, decision). | Anyone (Public query) | Any | N/A (Read-only) |
| `get_attestation(case_id, attestor) -> Option<Attestation>` | Queries registered attestation for a specific address on a case. | Anyone (Public query) | Any | N/A (Read-only) |
| `is_observer(observer: Address) -> bool` | Checks whether an address is currently a registered authorized observer. | Anyone (Public query) | Any | N/A (Read-only) |
| `get_resolution(case_id, resolver) -> Option<BytesN<32>>` | Queries submitted dispute resolution commitment for a specific address. | Anyone (Public query) | Any | N/A (Read-only) |

---

## Events

The contract emits typed Soroban events for all state-mutating transitions, allowing downstream indexers and services to react in real time:

| Event Name | Firing Condition & Entrypoint | Topics | Data Payload |
| :--- | :--- | :--- | :--- |
| `ObserverAdded` | Admin registers a new observer (`add_observer`). | `observer: Address` | `()` |
| `ObserverRemoved` | Admin revokes an observer (`remove_observer`). | `observer: Address` | `()` |
| `CaseCreated` | Case owner opens a new settlement case (`create_case`). | `case_id: BytesN<32>` | `owner`, `counterparty`, `expires_at_ledger` |
| `ObservationRecorded` | Observer anchors payment evidence on open case (`record_observation`). | `case_id: BytesN<32>` | `observer`, `tx_hash`, `observed_ledger` |
| `CaseMatched` | Observer records reconciliation match (`record_match`). | `case_id: BytesN<32>` | `observer` |
| `CaseBroken` | Observer records reconciliation break (`record_break`). | `case_id: BytesN<32>` | `observer`, `break_code` |
| `AttestationSubmitted` | Participant submits cryptographic attestation (`submit_attestation`). | `case_id: BytesN<32>` | `attestor`, `role` |
| `DisputeOpened` | Owner or Counterparty opens dispute on broken case (`open_dispute`). | `case_id: BytesN<32>` | `initiator`, `dispute_commitment` |
| `ResolutionSubmitted` | Disputant submits proposed resolution terms (`submit_resolution`). | `case_id: BytesN<32>` | `resolver`, `resolution_commitment` |
| `DisputeResolved` | Both disputants independently submit matching terms (`submit_resolution`). | `case_id: BytesN<32>` | `resolution_commitment` |
| `CaseFinalized` | Owner finalizes case with required attestations (`finalize_case`). | `case_id: BytesN<32>` | `finalized_at_ledger` |

---

## Break Codes

When reconciliation fails, registered observers classify discrepancies using standardized `BreakCode` enum variants:

| Break Code | Meaning |
| :--- | :--- |
| `AmountMismatch` | The transferred payment amount differs from the expected amount in settlement terms. |
| `AssetMismatch` | The asset/token delivered does not match the agreed settlement currency or issuer. |
| `DestinationMismatch` | The transaction paid an address other than the specified recipient. |
| `ReferenceMismatch` | The payment memo, invoice ID, or reference tag does not match trade instructions. |
| `MissingSettlement` | No matching payment transaction was observed on the ledger prior to expiration. |
| `DuplicateSettlement` | Multiple conflicting payment transactions were observed for the same case ID. |
| `LateSettlement` | The payment occurred after the agreed expiration ledger sequence. |
| `FailedTransaction` | The observed Stellar transaction failed or reverted on the ledger. |
| `UnexpectedTransaction` | An unrequested or unauthorized payment was attributed to the case reference. |

---

## Authorization & Trust Model

### Role Separation Matrix

| Role | Permissions & Authority | Prohibited Actions / Boundaries |
| :--- | :--- | :--- |
| **Contract Admin** | Manages authorized observer whitelist (`add_observer`, `remove_observer`). | **CANNOT** create cases, record observations, alter decisions, force settlements, submit attestations, or mutate case data. |
| **Case Owner** | Creates case (`create_case`), submits owner attestation, opens dispute, submits resolution terms, and finalizes case (`finalize_case`). | Cannot record observations or decisions unless also a registered observer. Cannot finalize without required attestations. |
| **Counterparty** | Submits counterparty attestation, opens dispute, and submits resolution terms. | Cannot finalize case or bypass resolution consensus. |
| **Observer** | Records Stellar transaction evidence (`record_observation`), records reconciliation matches/breaks (`record_match`, `record_break`), and submits observer attestations. | Cannot alter case parameters or finalize cases. Must be registered on-chain by admin. |
| **Public (Anyone)** | Query read-only endpoints (`get_case`, `get_attestation`, `is_observer`, `get_resolution`). | Cannot execute state-mutating transitions. |

### Administrative Scope & Immutability Guarantees

1. **Restricted Admin Powers**: The contract administrator key is strictly scoped to observer registry management. Admin keys cannot mutate case records, bypass state transitions, or force outcomes.
2. **Terminal Immutability**: Once a case reaches `Finalized`, all state-mutating endpoints explicitly revert with `Error::InvalidState`. No administrative override exists.
3. **Cryptographic Integrity**: SHA-256 hashes (`BytesN<32>`) represent deterministic off-chain commitments to private business documents, preventing on-chain data leakage while ensuring tamper-proof verification.
4. **Observer Trust Assumption**: Observers are trusted to accurately ingest Stellar ledger payment activity and record honest match/break decisions.

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
- **Protocol Monorepo**: [StellarClear App, SDK, API & Indexer](https://github.com/StellarClear/stellarclear-app)

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
