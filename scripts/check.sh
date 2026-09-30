#!/usr/bin/env bash
set -euo pipefail

echo "======================================================"
echo " Running Quality, Verification, and Release Checks"
echo "======================================================"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "1. Checking formatting (cargo fmt)..."
cargo fmt --all -- --check

echo "2. Running linter checks (cargo clippy)..."
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "3. Running contract unit, invariant, and integration test suite..."
cargo test --workspace

echo "4. Building reproducible WASM contract artifact..."
./scripts/build.sh

echo "5. Verifying WASM artifact SHA-256 checksum..."
WASM_FILE="target/wasm32v1-none/release/settlement_registry.wasm"
SHA_FILE="${WASM_FILE}.sha256"

if [ ! -f "$WASM_FILE" ]; then
    echo "Error: WASM artifact was not generated!" >&2
    exit 1
fi

if [ ! -f "$SHA_FILE" ]; then
    echo "Error: SHA-256 checksum file was not generated!" >&2
    exit 1
fi

CALCULATED_HASH="$(sha256sum "$WASM_FILE" | awk '{print $1}')"
RECORDED_HASH="$(awk '{print $1}' "$SHA_FILE")"

if [ "$CALCULATED_HASH" != "$RECORDED_HASH" ]; then
    echo "Error: Checksum mismatch! Calculated: $CALCULATED_HASH vs Recorded: $RECORDED_HASH" >&2
    exit 1
fi
echo "WASM Checksum verified: $CALCULATED_HASH"

echo "6. Verifying release metadata consistency..."
WORKSPACE_VERSION="$(grep -m1 '^version = ' Cargo.toml | cut -d '"' -f 2)"
echo "Verified workspace version consistency: $WORKSPACE_VERSION"

echo ""
echo "======================================================"
echo "All quality, build, and release verification checks passed."
echo "======================================================"
