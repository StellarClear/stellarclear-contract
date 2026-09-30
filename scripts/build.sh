#!/usr/bin/env bash
set -euo pipefail

echo "======================================================"
echo " Building StellarClear SettlementRegistry Contract"
echo "======================================================"

# Determine project root
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Check required target
if ! rustup target list --installed | grep -q "wasm32v1-none"; then
    echo "Adding wasm32v1-none compilation target..."
    rustup target add wasm32v1-none
fi

echo "Building contract via stellar CLI..."
if command -v stellar &>/dev/null; then
    stellar contract build
else
    echo "stellar CLI not found in PATH, falling back to cargo rustc..."
    cargo rustc \
        --manifest-path contracts/settlement-registry/Cargo.toml \
        --crate-type cdylib \
        --target wasm32v1-none \
        --release
fi

WASM_FILE="target/wasm32v1-none/release/settlement_registry.wasm"

if [ -f "$WASM_FILE" ]; then
    echo ""
    echo "======================================================"
    echo " Contract Artifact Metadata"
    echo "======================================================"
    echo "Artifact Path: $WASM_FILE"
    echo "Artifact Size: $(wc -c < "$WASM_FILE") bytes"
    if command -v sha256sum &>/dev/null; then
        echo "SHA-256 Hash: $(sha256sum "$WASM_FILE" | awk '{print $1}')"
    elif command -v shasum &>/dev/null; then
        echo "SHA-256 Hash: $(shasum -a 256 "$WASM_FILE" | awk '{print $1}')"
    fi
    echo "======================================================"
    echo "Build succeeded."
else
    echo "Error: WASM artifact not found at $WASM_FILE" >&2
    exit 1
fi
