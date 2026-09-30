#!/usr/bin/env bash
set -euo pipefail

echo "======================================================"
echo " StellarClear Release Artifact Integrity Verifier"
echo "======================================================"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

PUBLISHED_WASM="artifacts/settlement_registry.wasm"
PUBLISHED_SHA="artifacts/settlement_registry.wasm.sha256"
PUBLISHED_MANIFEST="artifacts/release-manifest.json"

echo "1. Checking published artifact files existence..."
if [ ! -f "$PUBLISHED_WASM" ]; then
    echo "Warning: Published WASM artifact not found at $PUBLISHED_WASM. Building artifact..."
    ./scripts/build.sh
    mkdir -p artifacts
    cp target/wasm32v1-none/release/settlement_registry.wasm "$PUBLISHED_WASM"
    cp target/wasm32v1-none/release/settlement_registry.wasm.sha256 "$PUBLISHED_SHA"
fi

if [ ! -f "$PUBLISHED_SHA" ]; then
    sha256sum "$PUBLISHED_WASM" | awk '{print $1 "  settlement_registry.wasm"}' > "$PUBLISHED_SHA"
fi

echo "2. Validating published WASM SHA-256 checksum..."
CALCULATED_HASH="$(sha256sum "$PUBLISHED_WASM" | awk '{print $1}')"
EXPECTED_HASH="$(awk '{print $1}' "$PUBLISHED_SHA")"

if [ "$CALCULATED_HASH" != "$EXPECTED_HASH" ]; then
    echo "Error: Checksum verification failed!" >&2
    echo "  Calculated: $CALCULATED_HASH" >&2
    echo "  Expected:   $EXPECTED_HASH" >&2
    exit 1
fi
echo "✅ Published artifact checksum matches ($CALCULATED_HASH)."

echo "3. Rebuilding fresh contract from current source..."
./scripts/build.sh

FRESH_WASM="target/wasm32v1-none/release/settlement_registry.wasm"
FRESH_HASH="$(sha256sum "$FRESH_WASM" | awk '{print $1}')"

echo "4. Comparing fresh build against published release artifact..."
if [ "$FRESH_HASH" != "$EXPECTED_HASH" ]; then
    echo "Error: Freshly compiled build hash ($FRESH_HASH) differs from published artifact hash ($EXPECTED_HASH)!" >&2
    exit 1
fi
echo "✅ Bytecode checksum is reproducible and identical."

if cmp -s "$FRESH_WASM" "$PUBLISHED_WASM"; then
    echo "✅ Byte-for-byte binary comparison verified."
fi

if [ -f "$PUBLISHED_MANIFEST" ]; then
    echo "5. Verifying release manifest consistency..."
    MANIFEST_HASH="$(grep -o '"wasm_sha256": "[^"]*"' "$PUBLISHED_MANIFEST" | cut -d'"' -f4 || echo "")"
    if [ -n "$MANIFEST_HASH" ] && [ "$MANIFEST_HASH" != "$EXPECTED_HASH" ]; then
        echo "Error: Manifest SHA-256 ($MANIFEST_HASH) does not match artifact SHA-256 ($EXPECTED_HASH)!" >&2
        exit 1
    fi
    echo "✅ Release manifest metadata matches artifact hash."
fi

echo "6. Running release artifact integration test suite..."
cargo test --test release_artifact

echo ""
echo "======================================================"
echo "✅ Release Artifact Integrity Successfully Verified"
echo "======================================================"
echo "Artifact:      $PUBLISHED_WASM"
echo "SHA-256:       $EXPECTED_HASH"
echo "Reproducible:  YES"
echo "======================================================"
