# Deployment and Verification Guide

This document details the end-to-end procedure for building, testing, deploying, initializing, and verifying the `SettlementRegistry` Soroban smart contract on the Stellar network.

---

## ⚠️ Security Status & Disclaimer

> [!CAUTION]
> **UNAUDITED SMART CONTRACT / PROTOTYPE**:
> The `SettlementRegistry` smart contract is under active development and has **not** undergone an independent third-party security audit or formal verification. Do not deploy or use this contract in production environments handling real financial value without prior independent security auditing.

---

## 1. Prerequisites & Environment Setup

### Required Tools

| Tool | Minimum Version | Description |
| :--- | :--- | :--- |
| **Rust** | `1.84.0+` | Rust compiler toolchain |
| **Target** | `wasm32v1-none` | Soroban WebAssembly compilation target |
| **Stellar CLI** | `28.1.0+` | Official Stellar command-line client |
| **Soroban SDK** | `27.0.4+` | Soroban environment SDK |

### Target Installation

```bash
rustup target add wasm32v1-none
```

### Stellar CLI Installation

```bash
cargo install --locked stellar-cli --version 28.1.0
```

---

## 2. Quality Checks & Reproducible Build

Before deploying to any network, execute the full test and verification suite:

```bash
# Run formatting, clippy linter, and unit/invariant tests
./scripts/check.sh
```

Build the optimized contract artifact and inspect the checksum:

```bash
./scripts/build.sh
```

### Artifact Details

- **Wasm Artifact Path**: `target/wasm32v1-none/release/settlement_registry.wasm`
- **Verify Checksum**:
  ```bash
  sha256sum target/wasm32v1-none/release/settlement_registry.wasm
  ```

---

## 3. Network Configuration

### Network Profiles

| Parameter | Testnet | Mainnet |
| :--- | :--- | :--- |
| **RPC URL** | `https://soroban-testnet.stellar.org` | `https://mainnet.sorobanrpc.com` |
| **Network Passphrase** | `Test SDF Network ; September 2015` | `Public Global Stellar Network ; July 2015` |
| **Network Flag** | `--network testnet` | `--network mainnet` |

### Configure CLI Networks & Identities

```bash
# Configure Testnet network profile
stellar network add \
  --global testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"

# Generate or import deployer identity
stellar keys generate deployer --network testnet

# Fund deployer identity on Testnet via Friendbot
stellar keys fund deployer --network testnet
```

---

## 4. Contract Deployment & Initialization

The `SettlementRegistry` contract uses a constructor (`__constructor`) that requires the initial contract admin address upon deployment.

### Deploy to Testnet

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/settlement_registry.wasm \
  --source deployer \
  --network testnet \
  -- \
  --admin $(stellar keys address deployer)
```

The CLI will output the deployed **Contract ID** (e.g., `CA...` or `CB...`).

### Deploy to Mainnet

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/settlement_registry.wasm \
  --source deployer \
  --network mainnet \
  -- \
  --admin <PRODUCTION_ADMIN_ADDRESS>
```

---

## 5. Contract ID Recording & Integration Configuration

Record the deployed contract ID and configure downstream services (API, SDK, Indexer) via environment variables:

```bash
# Stellar network & RPC configuration
export STELLAR_RPC_URL="https://soroban-testnet.stellar.org"
export STELLAR_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"

# SettlementRegistry contract identifier
export SETTLEMENT_REGISTRY_CONTRACT_ID="<DEPLOYED_CONTRACT_ID>"

# Admin / Deployer identity for administrative operations
export SETTLEMENT_REGISTRY_ADMIN_ADDRESS="<ADMIN_ADDRESS>"
```

Save these variables into your deployment configuration or secret management vault.

---

---

## 6. Post-Deployment & Testnet Lifecycle Verification

Verify contract state, event emissions, and complete settlement lifecycle transitions after deployment on Soroban Testnet:

### Automated Lifecycle Verification Scripts

Execute the end-to-end automated verification against Soroban testnet:

```bash
# Automated build, deploy, lifecycle transitions, and event checks
./scripts/testnet-verification.sh

# Verify an existing deployed contract ID
./scripts/verify-deployment.sh <CONTRACT_ID> testnet
```

### Programmatic Integration Test Matrix

Execute the dedicated Soroban integration test suite validating the full deployment lifecycle:

```bash
cargo test --test deployed_testnet -- --nocapture
```

The integration test matrix verifies:
1. **Network & Contract Capture**: Explicit recording of network passphrase (`Test SDF Network ; September 2015`) and deployed contract address.
2. **Observer Management**: Admin adds observer (`add_observer`), verifies registration (`is_observer`), and checks `ObserverAdded` event emission.
3. **Case Lifecycle (Match Path)**:
   - Case creation (`create_case`) emitting `CaseCreated`.
   - Observation recording (`record_observation`) with transaction hash and ledger index emitting `ObservationRecorded`.
   - Reconciliation match (`record_match`) transitioning state from `Observed` to `Matched` emitting `CaseMatched`.
   - Three-party attestation submission (`submit_attestation`) by Owner, Counterparty, and Observer emitting `AttestationSubmitted`.
   - Case finalization (`finalize_case`) transitioning state to `Finalized` emitting `CaseFinalized`.
4. **Case Lifecycle (Break & Dispute Path)**:
   - Observation recording followed by `record_break` with typed `BreakCode` (emitting `CaseBroken`).
   - Dispute opening (`open_dispute`) by owner emitting `DisputeOpened`.
   - Two-party resolution agreement (`submit_resolution`) transitioning state to `Resolved` emitting `DisputeResolved`.
   - Attestation and finalization on resolved dispute cases.
5. **Bytecode Execution Verification**: Loads compiled WASM artifact from `target/wasm32v1-none/release/settlement_registry.wasm`, validates SHA-256 checksum, and verifies bytecode execution fidelity in Soroban host environment.

---

## 7. Upgrade & Change Process

### Immutability & Upgrade Policy

1. **State Immutability**: All finalized settlement cases, terms commitments, and reconciliation break records are permanently immutable on-chain.
2. **Contract Instance Versioning**:
   - The current architecture favors deploying new versioned contract instances for major protocol revisions.
   - Downstream indexers and API services reference new contract IDs while retaining historical query access to existing contract instances.
3. **Emergency Controls**:
   - There are **no backdoor administrative state mutations** or fund confiscation capabilities.
   - Admin authority is strictly limited to observer authorization management (`add_observer`, `remove_observer`).

---

## 8. Summary Checklist

- [ ] All unit, auth, adversarial, and boundary tests pass (`./scripts/check.sh`).
- [ ] Contract Wasm built cleanly and SHA-256 hash recorded (`./scripts/build.sh`).
- [ ] Contract deployed with explicit `--admin` parameter.
- [ ] Contract ID recorded in deployment environment (`SETTLEMENT_REGISTRY_CONTRACT_ID`).
- [ ] Initial observer address(es) registered via `add_observer`.
- [ ] Deployed testnet verification executed (`./scripts/testnet-verification.sh` & `cargo test --test deployed_testnet`).
- [ ] Smoke test case created and queried on target network.
- [ ] Final on-chain state readability verified (`./scripts/verify-deployment.sh`).
