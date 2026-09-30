# SettlementRegistry Release Artifacts

This directory contains versioned, reproducible compiled WebAssembly (`.wasm`) artifacts, SHA-256 checksums, and release manifests for the `SettlementRegistry` Soroban smart contract.

---

## Directory Layout

```text
artifacts/
├── README.md                          # Documentation and verification guide
├── release-manifest.json              # Active/latest release metadata manifest
├── settlement_registry.wasm           # Latest compiled WASM binary artifact
├── settlement_registry.wasm.sha256    # SHA-256 checksum for verification
└── v0.1.0/                            # Version-specific archived release directory
    ├── release-manifest.json          # v0.1.0 release metadata manifest
    ├── settlement_registry.wasm       # v0.1.0 WASM binary
    └── settlement_registry.wasm.sha256# v0.1.0 SHA-256 checksum
```

---

## Artifact Packaging & Generation

Release artifacts are produced reproducibly via:

```bash
./scripts/release.sh
```

This automates:
1. Building the optimized WASM artifact with deterministic compiler flags.
2. Generating the SHA-256 cryptographic checksum.
3. Extracting metadata (Git commit, branch, rustc version, stellar-cli version, SDK version).
4. Generating `release-manifest.json`.
5. Creating versioned directory snapshots.

---

## Verification

To verify the integrity of the release artifact:

```bash
# Verify checksum matches artifact byte-for-byte
sha256sum -c artifacts/settlement_registry.wasm.sha256
```

---

## Release Manifest Schema

The `release-manifest.json` provides machine-readable metadata:

```json
{
  "contract_name": "settlement_registry",
  "version": "0.1.0",
  "git_commit": "ac3d68390d302e498fea39fc8e93fcf680e7ad00",
  "git_branch": "main",
  "timestamp": "2026-09-30T05:49:19Z",
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
