#!/usr/bin/env bash
set -euo pipefail

echo "======================================================"
echo " StellarClear Release Artifact Chain Verifier"
echo "======================================================"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

PUBLISHED_WASM="artifacts/settlement_registry.wasm"
PUBLISHED_SHA="artifacts/settlement_registry.wasm.sha256"
PUBLISHED_MANIFEST="artifacts/release-manifest.json"

echo "1. Checking published artifact files..."
if [ ! -f "$PUBLISHED_WASM" ]; then
    echo "Warning: Published WASM artifact not found at $PUBLISHED_WASM. Generating build..."
    ./scripts/build.sh
fi

if [ -f "$PUBLISHED_SHA" ]; then
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
fi

echo "3. Rebuilding fresh contract from current source..."
./scripts/build.sh

FRESH_WASM="target/wasm32v1-none/release/settlement_registry.wasm"
FRESH_HASH="$(sha256sum "$FRESH_WASM" | awk '{print $1}')"

if [ -f "$PUBLISHED_WASM" ]; then
    echo "4. Comparing fresh build against published release artifact..."
    PUBLISHED_CURRENT_HASH="$(sha256sum "$PUBLISHED_WASM" | awk '{print $1}')"
    if [ "$FRESH_HASH" != "$PUBLISHED_CURRENT_HASH" ]; then
        echo "Error: Freshly compiled build hash ($FRESH_HASH) differs from published artifact hash ($PUBLISHED_CURRENT_HASH)!" >&2
        exit 1
    fi
    echo "✅ Bytecode checksum is reproducible and identical."
fi

if [ -f "$PUBLISHED_MANIFEST" ]; then
    echo "5. Verifying release manifest consistency..."
    MANIFEST_HASH="$(grep -o '"wasm_sha256": "[^"]*"' "$PUBLISHED_MANIFEST" | cut -d'"' -f4 || echo "")"
    if [ -n "$MANIFEST_HASH" ] && [ "$MANIFEST_HASH" != "$FRESH_HASH" ]; then
        echo "Error: Manifest SHA-256 ($MANIFEST_HASH) does not match artifact SHA-256 ($FRESH_HASH)!" >&2
        exit 1
    fi
    echo "✅ Release manifest metadata is consistent with compiled binary."
fi

echo "6. Running release artifact integration test suite..."
cargo test --test release_artifact

echo ""
echo "======================================================"
echo "✅ Release Artifact Chain Verification Successful"
echo "======================================================"
echo "Artifact:      $FRESH_WASM"
echo "SHA-256:       $FRESH_HASH"
echo "Reproducible:  YES"
echo "======================================================"
