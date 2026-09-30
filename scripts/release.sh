#!/usr/bin/env bash
set -euo pipefail

echo "======================================================"
echo " StellarClear Contract Release Artifact Packager"
echo "======================================================"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# 1. Ensure build succeeds
./scripts/build.sh

VERSION="$(grep -m1 '^version = ' Cargo.toml | cut -d '"' -f 2)"
GIT_COMMIT="$(git rev-parse HEAD 2>/dev/null || echo "unknown")"
GIT_BRANCH="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
RUST_VERSION="$(rustc --version 2>/dev/null || echo "unknown")"
STELLAR_VERSION="$(stellar --version 2>/dev/null | head -n 1 || echo "unknown")"

SRC_WASM="target/wasm32v1-none/release/settlement_registry.wasm"
if [ ! -f "$SRC_WASM" ]; then
    echo "Error: WASM artifact not found at $SRC_WASM" >&2
    exit 1
fi

WASM_SIZE="$(wc -c < "$SRC_WASM")"
WASM_HASH="$(sha256sum "$SRC_WASM" | awk '{print $1}')"

# 2. Package artifacts
ARTIFACTS_DIR="artifacts"
VERSIONED_DIR="artifacts/v${VERSION}"
mkdir -p "$ARTIFACTS_DIR" "$VERSIONED_DIR"

cp "$SRC_WASM" "$ARTIFACTS_DIR/settlement_registry.wasm"
cp "$SRC_WASM" "$VERSIONED_DIR/settlement_registry.wasm"

echo "$WASM_HASH  settlement_registry.wasm" > "$ARTIFACTS_DIR/settlement_registry.wasm.sha256"
echo "$WASM_HASH  settlement_registry.wasm" > "$VERSIONED_DIR/settlement_registry.wasm.sha256"

# 3. Generate Release Manifest JSON
MANIFEST_FILE="$ARTIFACTS_DIR/release-manifest.json"
cat <<EOF > "$MANIFEST_FILE"
{
  "contract_name": "settlement_registry",
  "version": "$VERSION",
  "git_commit": "$GIT_COMMIT",
  "git_branch": "$GIT_BRANCH",
  "timestamp": "$TIMESTAMP",
  "wasm_file": "settlement_registry.wasm",
  "wasm_sha256": "$WASM_HASH",
  "wasm_size_bytes": $WASM_SIZE,
  "rustc_version": "$RUST_VERSION",
  "stellar_cli_version": "$STELLAR_VERSION",
  "soroban_sdk_version": "27.0.4",
  "target": "wasm32v1-none",
  "networks": {
    "testnet": {
      "rpc_url": "https://soroban-testnet.stellar.org",
      "network_passphrase": "Test SDF Network ; September 2015",
      "contract_id": "${SETTLEMENT_REGISTRY_TESTNET_ID:-}"
    },
    "mainnet": {
      "rpc_url": "https://mainnet.sorobanrpc.com",
      "network_passphrase": "Public Global Stellar Network ; July 2015",
      "contract_id": "${SETTLEMENT_REGISTRY_MAINNET_ID:-}"
    }
  }
}
EOF

cp "$MANIFEST_FILE" "$VERSIONED_DIR/release-manifest.json"

echo ""
echo "======================================================"
echo " Release Artifacts Packaged Successfully"
echo "======================================================"
echo "Version:         $VERSION"
echo "Commit:          $GIT_COMMIT"
echo "WASM Artifact:   $ARTIFACTS_DIR/settlement_registry.wasm"
echo "SHA-256 Hash:    $WASM_HASH"
echo "Checksum File:   $ARTIFACTS_DIR/settlement_registry.wasm.sha256"
echo "Manifest:        $MANIFEST_FILE"
echo "Versioned Dir:   $VERSIONED_DIR"
echo "======================================================"
