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

## 6. Post-Deployment Verification

Verify contract state and functionality after deployment:

### 1. Verification Scripts

```bash
# Verify deployed contract on network against release artifact
./scripts/verify-deployment.sh <SETTLEMENT_REGISTRY_CONTRACT_ID> testnet
```

### 2. Register an Observer

```bash
stellar contract invoke \
  --id $SETTLEMENT_REGISTRY_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  add_observer \
  --observer <OBSERVER_ADDRESS>
```

### 3. Verify Observer Registration

```bash
stellar contract invoke \
  --id $SETTLEMENT_REGISTRY_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  is_observer \
  --observer <OBSERVER_ADDRESS>
```
*Expected Output: `true`*

### 4. Smoke Test Case Creation

```bash
# Create a test settlement case
stellar contract invoke \
  --id $SETTLEMENT_REGISTRY_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  create_case \
  --case_id "0101010101010101010101010101010101010101010101010101010101010101" \
  --owner $(stellar keys address deployer) \
  --counterparty "[]" \
  --terms_commitment "0202020202020202020202020202020202020202020202020202020202020202" \
  --expires_at_ledger 99999999
```

### 5. Query Case State

```bash
stellar contract invoke \
  --id $SETTLEMENT_REGISTRY_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  get_case \
  --case_id "0101010101010101010101010101010101010101010101010101010101010101"
```
*Expected Output: Case object with status `0` (`Open`).*

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

- [ ] All unit, auth, adversarial, boundary, and fuzz tests pass (`./scripts/check.sh`).
- [ ] Contract Wasm built cleanly and SHA-256 hash recorded (`./scripts/build.sh`).
- [ ] Release artifact chain verified (`./scripts/verify-artifact.sh`).
- [ ] Contract deployed with explicit `--admin` parameter.
- [ ] Contract ID recorded in deployment environment (`SETTLEMENT_REGISTRY_CONTRACT_ID`).
- [ ] Initial observer address(es) registered via `add_observer`.
- [ ] Deployed contract verified against release specification (`./scripts/verify-deployment.sh`).
- [ ] Full release-candidate checklist verified in [`RELEASE.md`](./RELEASE.md).
