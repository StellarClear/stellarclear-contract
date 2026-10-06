# StellarClear Settlement Registry (`stellarclear-contract`)

Soroban smart contract for settlement evidence, observer quorum, disputes, and finalization.

<p align="center">
  <img src="./docs/assets/banner.jpeg" alt="StellarClear Banner" width="100%" />
</p>

<p align="center">
  <a href="./LICENSE"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License"></a>
  <a href="https://github.com/StellarClear/stellarclear-contract/actions/workflows/ci.yml"><img src="https://github.com/StellarClear/stellarclear-contract/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/StellarClear/stellarclear-contract/actions/workflows/security-verification.yml"><img src="https://github.com/StellarClear/stellarclear-contract/actions/workflows/security-verification.yml/badge.svg" alt="Security Verification"></a>
  <a href="https://github.com/StellarClear/stellarclear-contract/actions/workflows/release-verification.yml"><img src="https://github.com/StellarClear/stellarclear-contract/actions/workflows/release-verification.yml/badge.svg" alt="Release Verification"></a>
  <a href="https://github.com/StellarClear/stellarclear-contract/releases/tag/v0.1.1"><img src="https://img.shields.io/badge/Release-v0.1.1-brightgreen.svg" alt="Release v0.1.1"></a>
  <a href="https://stellar.org/soroban"><img src="https://img.shields.io/badge/Soroban-v27.0-7023e0.svg" alt="Soroban"></a>
</p>

---

## Quick Links

- [Application Repository](https://github.com/StellarClear/stellarclear-app)
- [Application Documentation](https://stellarclear.github.io/stellarclear-app/)
- [Application Releases](https://github.com/StellarClear/stellarclear-app/releases)
- [Contract Release (v0.1.1)](https://github.com/StellarClear/stellarclear-contract/releases/tag/v0.1.1)
- [Active Testnet Contract](https://stellar.expert/explorer/testnet/contract/CCPCMPIUTKBLSJVSGPPSUHTBBY6HHT3OTE3CKUXC5BD2B3YEDSAROGC5)
- [StellarExpert Contract Explorer](https://stellar.expert/explorer/testnet/contract/CCPCMPIUTKBLSJVSGPPSUHTBBY6HHT3OTE3CKUXC5BD2B3YEDSAROGC5)
- [Security Policy (`SECURITY.md`)](./SECURITY.md)
- [Deployment Guide (`DEPLOYMENT.md`)](./DEPLOYMENT.md)
- [Contributing Guidelines (`CONTRIBUTING.md`)](./CONTRIBUTING.md)

---

## Table of Contents

- [Contract Purpose](#contract-purpose)
- [Protocol Architecture & Boundary](#protocol-architecture--boundary)
- [State Machine](#state-machine)
- [Pure M-of-N Observer Quorum](#pure-m-of-n-observer-quorum)
- [Dispute Expiration & Ledger-Based TTL](#dispute-expiration--ledger-based-ttl)
- [Public Interface Reference](#public-interface-reference)
- [Protocol Events](#protocol-events)
- [Reconciliation Break Codes](#reconciliation-break-codes)
- [Testnet Deployment](#testnet-deployment)
- [Build and Verification](#build-and-verification)
- [Security & Invariants](#security--invariants)
- [StellarClear Application](#stellarclear-application)
- [Maintainers & Support](#maintainers--support)
- [Contributing](#contributing)
- [License](#license)

---

## Contract Purpose

`SettlementRegistry` is the authoritative on-chain protocol layer for StellarClear on Stellar/Soroban.

The contract records and governs:
- **Settlement Cases**: Case creation, cryptographic terms commitments, and counterparties.
- **Observations**: Stellar transaction hashes, ledger sequence numbers, and observation commitments.
- **Match / Break Decisions**: Positive reconciliation matches or typed reconciliation breaks (`BreakCode`).
- **Owner & Counterparty Attestations**: Cryptographic commitments submitted by primary trade principals.
- **Observer Attestations**: Cryptographic evidence submitted by whitelisted observer addresses.
- **Quorum Configuration**: Per-case threshold configuration (`M`-of-`N`).
- **Disputes**: Formal dispute initiation on broken cases with ledger-based TTL.
- **Resolution Commitments**: Dual-party agreed terms resolving active disputes.
- **Irreversible Finalization**: Terminal locking of matched or resolved settlement outcomes.

**Non-Custodial Design**: The smart contract **does NOT custody user funds**, escrow balances, or execute asset transfers. Value transfers occur on underlying Stellar payment rails.

---

## Protocol Architecture & Boundary

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
│ - Pure M-of-N Quorum & Irreversible Finalization        │
└─────────────────────────────────────────────────────────┘
```

---

## State Machine

Settlement cases follow an explicit, deterministic state progression:

```text
Open ──► Observed ──┬──► Matched ─────────────────────────────────► Finalized
                    │
                    └──► Break ──► Disputed ──► Resolved ─────────► Finalized
                           ▲          │
                           └──(TTL)───┘ (expire_dispute)
```

### Supported Transitions:
1. **Happy Path (Direct Reconciliation Match)**:
   ```text
   Open -> Observed -> Matched -> Finalized
   ```
   - `Open -> Observed`: Triggered by `record_observation` from an authorized observer.
   - `Observed -> Matched`: Triggered by `record_match` from an authorized observer.
   - `Matched -> Finalized`: Triggered by `finalize_case` from the case owner once Owner attestation and the required M-of-N observer quorum attestations are recorded.

2. **Dispute & Mutual Resolution Path**:
   ```text
   Observed -> Break -> Disputed -> Resolved -> Finalized
   ```
   - `Observed -> Break`: Triggered by `record_break` with a typed `BreakCode` from an authorized observer.
   - `Break -> Disputed`: Triggered by `open_dispute` or `open_dispute_with_ttl` from the case owner or counterparty.
   - `Disputed -> Resolved`: Triggered by `submit_resolution` when both owner and counterparty independently submit identical 32-byte resolution commitments prior to dispute TTL expiration.
   - `Resolved -> Finalized`: Triggered by `finalize_case` from the case owner once Owner, Counterparty, and M-of-N observer quorum attestations are recorded.

3. **Dispute Expiration / Timeout Path**:
   ```text
   Disputed -> (TTL Expiry) -> Break
   ```
   - `Disputed -> Break`: Triggered by permissionless `expire_dispute` once the current ledger sequence reaches or exceeds `dispute_expires_at_ledger`.
   - **Critical Invariant**: A dispute timeout **never transitions to `Resolved`** and **never fabricates mutual agreement**. It resets the case to `Break`, allowing parties to renegotiate or open a new dispute.

---

## Pure M-of-N Observer Quorum

The protocol implements a pure M-of-N observer quorum model:

- **Any M Distinct Valid Observers**: For a case requiring quorum threshold $M$, any $M$ distinct valid observer attestations satisfy the quorum.
- **Original Observer Not Mandatory**: The observer that submitted the initial `record_observation` has no special finalization privilege and is not mandatory for finalization.
- **Role Separation**: Case Owner and Counterparty cannot submit observer attestations or count toward observer quorum.
- **Deduplication**: Duplicate attestation submissions from the same observer count as a single attestation slot.
- **Historical Attestation Validity**: Observer authorization is evaluated at submission time (`submit_attestation` or `submit_observer_attestation`). Once recorded, an attestation represents immutable historical evidence and remains valid even if the observer is subsequently revoked by the admin.
- **Default Threshold**: Cases default to `quorum = 1` for backwards compatibility.

### Protocol Bounds:
- `MAX_OBSERVER_QUORUM = 10`: Maximum allowed observer quorum threshold per case.
- `MAX_OBSERVERS_PER_CASE = 16`: Maximum distinct observer attestations recorded per case.

---

## Dispute Expiration & Ledger-Based TTL

Dispute lifetimes are tracked via deterministic Stellar ledger sequence numbers, avoiding dependency on non-deterministic wall-clock timestamps:

- **Default Expiration**: `DEFAULT_DISPUTE_TTL_LEDGERS = 17_280` ledgers (~24 hours at 5 seconds/ledger) applied by `open_dispute`.
- **Custom Expiration**: Configurable via `open_dispute_with_ttl(initiator, case_id, dispute_commitment, ttl_ledgers)`.
- **TTL Bounds**:
  - `MIN_DISPUTE_TTL_LEDGERS = 120` ledgers (~10 minutes).
  - `MAX_DISPUTE_TTL_LEDGERS = 518_400` ledgers (~30 days).
- **Deterministic Timeout**: If `current_ledger >= dispute_expires_at_ledger` without matching resolution submissions, `expire_dispute` transitions the case deterministically from `Disputed -> Break`, clears pending resolution commitments, and emits `DisputeExpired`.

---

## Public Interface Reference

The `SettlementRegistry` contract exposes 22 public functions grouped by operational domain:

### Observer Registry (Admin & Queries)
| Function | Description | Callable By |
| :--- | :--- | :--- |
| `__constructor(admin: Address)` | One-time initialization setting the protocol admin and version `1`. | Deployer |
| `add_observer(observer: Address)` | Registers an authorized settlement observer address. | Contract Admin |
| `remove_observer(observer: Address)` | Revokes authorization for an existing settlement observer. | Contract Admin |
| `is_observer(observer: Address) -> bool` | Checks whether an address is currently an authorized observer. | Public (Read-only) |

### Case Creation & Configuration
| Function | Description | Callable By |
| :--- | :--- | :--- |
| `create_case(case_id, owner, counterparty, terms_commitment, expires_at_ledger)` | Opens a new settlement case (defaults quorum to 1). | Case Owner |
| `set_case_quorum(case_id, quorum)` | Configures observer quorum threshold (`1 <= quorum <= 10`). | Case Owner |
| `get_case_quorum(case_id) -> u32` | Reads configured observer quorum threshold for a case. | Public (Read-only) |

### Observation & Matching
| Function | Description | Callable By |
| :--- | :--- | :--- |
| `record_observation(observer, case_id, tx_hash, observed_ledger, observation_commitment)` | Anchors Stellar transaction evidence on open case. | Registered Observer |
| `record_match(observer, case_id)` | Records positive reconciliation match (`Observed -> Matched`). | Registered Observer |
| `record_break(observer, case_id, break_code)` | Records reconciliation break with standardized `BreakCode` (`Observed -> Break`). | Registered Observer |

### Attestations
| Function | Description | Callable By |
| :--- | :--- | :--- |
| `submit_attestation(case_id, role, commitment)` | Submits 32-byte cryptographic attestation (`Owner`, `Counterparty`, `Observer`). | Authorized Principal |
| `submit_observer_attestation(case_id, observer, commitment)` | Submits authorized observer attestation towards M-of-N quorum. | Registered Observer |
| `get_attested_observers(case_id) -> Vec<Address>` | Queries distinct observer addresses that submitted attestations for a case. | Public (Read-only) |
| `get_attestation(case_id, party) -> Option<Attestation>` | Queries registered attestation for a specific address. | Public (Read-only) |

### Disputes
| Function | Description | Callable By |
| :--- | :--- | :--- |
| `open_dispute(initiator, case_id, dispute_commitment)` | Opens dispute with default ~24h TTL on broken case. | Case Owner / Counterparty |
| `open_dispute_with_ttl(initiator, case_id, dispute_commitment, ttl_ledgers)` | Opens dispute with custom TTL ledgers (`120..=518,400`). | Case Owner / Counterparty |
| `expire_dispute(case_id)` | Permissionless timeout: resets unaddressed dispute back to `Break`. | Anyone (Permissionless) |
| `get_dispute_expiration(case_id) -> Option<u32>` | Reads expiration ledger sequence for an active dispute. | Public (Read-only) |

### Resolution & Finalization
| Function | Description | Callable By |
| :--- | :--- | :--- |
| `submit_resolution(party, case_id, resolution_commitment)` | Submits resolution terms; transitions to `Resolved` once both match. | Case Owner / Counterparty |
| `finalize_case(case_id)` | Irreversibly locks settlement outcome after verifying required role and quorum attestations. | Case Owner |

### General Queries
| Function | Description | Callable By |
| :--- | :--- | :--- |
| `get_case(case_id) -> SettlementCase` | Queries complete on-chain case record. | Public (Read-only) |
| `get_resolution(case_id, party) -> Option<BytesN<32>>` | Queries submitted dispute resolution commitment for a specific address. | Public (Read-only) |

---

## Protocol Events

The contract emits typed Soroban events for all state mutations. Downstream indexers in [`StellarClear/stellarclear-app`](https://github.com/StellarClear/stellarclear-app) ingest these events to maintain real-time off-chain database synchronization:

| Event Name | Topics | Data Payload | Firing Entrypoint |
| :--- | :--- | :--- | :--- |
| `ObserverAdded` | `observer: Address` | `()` | `add_observer` |
| `ObserverRemoved` | `observer: Address` | `()` | `remove_observer` |
| `CaseCreated` | `case_id: BytesN<32>` | `owner, counterparty, expires_at_ledger` | `create_case` |
| `CaseQuorumSet` | `case_id: BytesN<32>` | `quorum: u32` | `set_case_quorum` |
| `ObservationRecorded` | `case_id: BytesN<32>` | `observer, tx_hash, observed_ledger` | `record_observation` |
| `CaseMatched` | `case_id: BytesN<32>` | `observer` | `record_match` |
| `CaseBroken` | `case_id: BytesN<32>` | `observer, break_code` | `record_break` |
| `AttestationSubmitted` | `case_id: BytesN<32>` | `attestor, role` | `submit_attestation` / `submit_observer_attestation` |
| `DisputeOpened` | `case_id: BytesN<32>` | `initiator, dispute_commitment` | `open_dispute` / `open_dispute_with_ttl` |
| `ResolutionSubmitted` | `case_id: BytesN<32>` | `resolver, resolution_commitment` | `submit_resolution` |
| `DisputeResolved` | `case_id: BytesN<32>` | `resolution_commitment` | `submit_resolution` |
| `DisputeExpired` | `case_id: BytesN<32>` | `expiration_ledger, closed_at_ledger` | `expire_dispute` |
| `CaseFinalized` | `case_id: BytesN<32>` | `finalized_at_ledger` | `finalize_case` |

---

## Reconciliation Break Codes

When payment reconciliation fails, registered observers classify discrepancies using typed `BreakCode` enum variants:

| Break Code | Meaning |
| :--- | :--- |
| `AmountMismatch` | Observed payment amount differs from settlement terms. |
| `AssetMismatch` | Asset/token delivered differs from agreed currency or issuer. |
| `DestinationMismatch` | Transaction paid an address other than the agreed recipient. |
| `ReferenceMismatch` | Payment memo, invoice ID, or reference tag does not match instructions. |
| `MissingSettlement` | No matching payment transaction was observed on the ledger before expiration. |
| `DuplicateSettlement` | Multiple conflicting payment transactions observed for the same case. |
| `LateSettlement` | Payment occurred after the agreed expiration ledger sequence. |
| `FailedTransaction` | Observed Stellar transaction failed or reverted on-chain. |
| `UnexpectedTransaction` | Unrequested or unauthorized payment attributed to case reference. |

---

## Testnet Deployment

### Active Hardened Testnet Deployment (v0.1.1)

| Parameter | Value |
| :--- | :--- |
| **Network** | Stellar Testnet |
| **Contract ID** | `CCPCMPIUTKBLSJVSGPPSUHTBBY6HHT3OTE3CKUXC5BD2B3YEDSAROGC5` |
| **Contract Explorer** | [StellarExpert Testnet Explorer](https://stellar.expert/explorer/testnet/contract/CCPCMPIUTKBLSJVSGPPSUHTBBY6HHT3OTE3CKUXC5BD2B3YEDSAROGC5) |
| **Release Tag** | [`v0.1.1`](https://github.com/StellarClear/stellarclear-contract/releases/tag/v0.1.1) |
| **Git Commit** | `2032666be97e8bf09ba9073952041ae3e4573ff1` |
| **WASM SHA-256** | `0073a4cb2027140ac34e4db6c64c2d4104590ec60cf0ef2e424909eba9ae36ac` |
| **WASM Size** | `32,773 bytes` (optimized, reproducible) |

### Historical Prototype (v0.1.0)
- **Release Tag**: [`v0.1.0`](https://github.com/StellarClear/stellarclear-contract/releases/tag/v0.1.0)
- **WASM SHA-256**: `1018a81b1ac95046cb00466ceda7ee347204c08b71b1c51b3c9611dd32215d66`
- **Contract ID**: `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`

> [!NOTE]
> The historical prototype (`1018a81b...`) corresponds strictly to the initial v0.1.0 milestone. The active, hardened protocol deployment is **v0.1.1** (`0073a4cb...`).

---

## Build and Verification

### Prerequisites
- **Rust**: `stable` (`1.84.0+` MSRV)
- **Target**: `wasm32v1-none`
- **Stellar CLI**: `28.1.0+`
- **Soroban SDK**: `27.0.4+`

### Build & Verification Commands

```bash
# 1. Check code formatting
cargo fmt --all -- --check

# 2. Run clippy linter with warnings treated as errors
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 3. Run all workspace unit, invariant, and integration tests
cargo test --workspace

# 4. Build optimized contract WASM binary
stellar contract build

# 5. Verify byte-for-byte reproducible artifact integrity
./scripts/verify-artifact.sh

# 6. Execute full quality check suite
./scripts/check.sh
```

---

## Security & Invariants

Please review our complete [Security Policy (`SECURITY.md`)](./SECURITY.md).

- **Unaudited Smart Contract**: The contract is a reference protocol implementation and has not yet undergone an independent third-party security audit. Do not use in production without independent professional auditing.
- **Non-Custodial**: The contract holds no user funds, token balances, or custody escrows.
- **On-Chain Authorization**: Strict role-based authentication (`require_auth()`, `require_admin_auth()`, `require_observer_auth()`) enforced at runtime.
- **Terminal Immutability**: Finalized cases permanently reject all mutating calls (`Error::InvalidState`). No administrative override or backdoors exist.

---

## StellarClear Application

The smart contract is the trust-minimized anchor of the larger StellarClear ecosystem:

- **`stellarclear-contract` (This Repo)**: Soroban protocol layer governing settlement state, observer quorum, attestations, disputes, and finalization.
- **`stellarclear-app` (Companion Repo)**: Comprehensive off-chain platform providing client SDKs, REST API, ingestion indexers, matching engine, proof generators, and database persistence.
  - **Repository**: [`StellarClear/stellarclear-app`](https://github.com/StellarClear/stellarclear-app)
  - **Documentation**: [https://stellarclear.github.io/stellarclear-app/](https://stellarclear.github.io/stellarclear-app/)
  - **Releases**: [Application Releases](https://github.com/StellarClear/stellarclear-app/releases)

---

## Maintainers & Support

| Name | GitHub | Role |
| :--- | :--- | :--- |
| **Adejumo** | [@Adejumo-2](https://github.com/Adejumo-2) | Lead Developer & Maintainer |
| **Smog** | [@smog123](https://github.com/smog123) | Core Contributor & Maintainer |

- **GitHub Discussions**: [Ask questions and participate](https://github.com/StellarClear/stellarclear-contract/discussions)
- **GitHub Issues**: [Report bugs or suggest features](https://github.com/StellarClear/stellarclear-contract/issues)

---

## Contributing

We welcome contributions from the community. Please review our [Contributing Guidelines (`CONTRIBUTING.md`)](./CONTRIBUTING.md) and [Code of Conduct (`CODE_OF_CONDUCT.md`)](./CODE_OF_CONDUCT.md) before submitting pull requests.

---

## License

Licensed under the Apache License, Version 2.0. See [`LICENSE`](./LICENSE) for details.
