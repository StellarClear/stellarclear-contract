# Security Policy

## Security Status & Production Disclaimer

> [!CAUTION]
> **UNAUDITED IMPLEMENTATION / DEVELOPMENT PROTOTYPE**:
> The smart contracts in this repository are under active development and have **NOT** undergone an independent third-party security audit or formal verification.
>
> This codebase is a reference protocol implementation and is strictly intended for testing, prototyping, and evaluation. **Do NOT deploy or use this contract in production environments handling real financial value without an independent professional security audit.**

---

## SettlementRegistry Security Invariants

The `SettlementRegistry` contract enforces strict role-based access control, cryptographic commitment validation, and state machine immutability.

### 1. Authorization Matrix

| Action | Authorized Role / Address | Preconditions & Validation |
| :--- | :--- | :--- |
| **Case Creation** (`create_case`) | **Case Owner** (`owner.require_auth()`) | Non-zero `case_id`, non-zero `terms_commitment`, `expires_at_ledger > current_ledger`, `counterparty != owner` |
| **Observer Registration** (`add_observer` / `remove_observer`) | **Contract Admin** (`require_admin_auth()`) | Single-admin control initialized at deployment |
| **Observation Recording** (`record_observation`) | **Registered Observer** (`require_observer_auth()`) | Case must be in `Open` state; non-zero `tx_hash` & `observation_commitment`; `0 < observed_ledger <= current_ledger` |
| **Reconciliation Matching** (`record_match`) | **Registered Observer** (`require_observer_auth()`) | Case must be in `Observed` state; sets status and decision to `Matched` |
| **Reconciliation Break** (`record_break`) | **Registered Observer** (`require_observer_auth()`) | Case must be in `Observed` state; standardized `BreakCode` recorded; sets status and decision to `Break` |
| **Attestation Submission** (`submit_attestation`) | **Role-Specific Authorized Principal** | `Owner`: requires `owner.require_auth()`; `Counterparty`: requires `counterparty.require_auth()`; `Observer`: requires active registered `observer.require_auth()` matching case observer; non-zero commitment; at most one attestation per role |
| **Dispute Opening** (`open_dispute`) | **Case Owner or Counterparty** (`initiator.require_auth()`) | Case must be in `Break` state; non-zero `dispute_commitment`; transitions state to `Disputed` |
| **Dispute Resolution** (`submit_resolution`) | **Case Owner or Counterparty** (`resolver.require_auth()`) | Case must be in `Disputed` state; non-zero `resolution_commitment`; transitions to `Resolved` **only when both parties submit identical commitments** |
| **Case Finalization** (`finalize_case`) | **Case Owner** (`owner.require_auth()`) | Case must be in `Matched` (requires Owner + Observer attestations) or `Resolved` (requires Owner + Counterparty + Observer attestations); observer must still be active |

### 2. State Machine & Immutability Invariants

- **Legal State Transitions**:
  - `Open` ──► `Observed`
  - `Observed` ──► `Matched`
  - `Observed` ──► `Break`
  - `Break` ──► `Disputed`
  - `Disputed` ──► `Resolved` (upon matching two-party resolution commitments)
  - `Matched` ──► `Finalized` (with Owner + Observer attestations)
  - `Resolved` ──► `Finalized` (with Owner + Counterparty + Observer attestations)
- **Terminal Immutability**:
  - Once a settlement case reaches `Finalized`, it enters a permanently terminal, read-only state.
  - No subsequent observations, reconciliation decisions, attestations, disputes, resolutions, or finalizations can mutate a finalized case.
  - The contract contains **no backdoors, administrative state overrides, or emergency mutation bypasses**.

---

## Protocol Security Assumptions

1. **Cryptographic Integrity**: SHA-256 commitments (`BytesN<32>`) represent deterministic, pre-image resistant commitments computed off-chain from canonical settlement terms, ledger transactions, or resolution agreements.
2. **Authorized Observer Honesty**: Registered observers are trusted to faithfully verify Stellar ledger transactions and accurately submit observation records.
3. **Soroban Ledger Determinism**: Ledger sequence numbers provided by the Soroban runtime environment are monotonic and tamper-proof.
4. **No Custodial Risk**: The contract deliberately holds no user funds, token balances, or escrow vaults. Value transfer occurs on underlying Stellar payment rails.

---

## Upgrade & Change Process

1. **Protocol Immutability**: All finalized settlement cases, terms commitments, and break records are permanently immutable on-chain.
2. **Contract Instance Versioning**: To upgrade protocol logic, new contract instances with updated Wasm code are deployed. Historical settlement evidence remains anchored and queryable on prior contract instances.
3. **Admin Scope**: Admin keys are strictly restricted to observer registry management (`add_observer`, `remove_observer`) and have no ability to alter case states, dispute records, or attestations.
4. **Release Checklist Compliance**: All releases must satisfy the mandatory pre-release and deployment verification checklist in [`RELEASE.md`](./RELEASE.md).

---

## Reporting Security Vulnerabilities

Please report security issues using **GitHub Private Vulnerability Reporting**:

1. Navigate to the repository's **Security** tab on GitHub.
2. Click on **Advisories** and select **Report a vulnerability**.
3. Provide detailed steps to reproduce the issue, including environment details, test cases, and the potential impact.

> [!NOTE]
> Do not disclose security vulnerabilities publicly in issues or discussions until they have been reviewed and addressed.

