#!/usr/bin/env bash
set -euo pipefail

echo "======================================================"
echo " Running Quality and Verification Checks"
echo "======================================================"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "1. Checking formatting (cargo fmt)..."
cargo fmt --all -- --check

echo "2. Running linter checks (cargo clippy)..."
cargo clippy --all-targets --all-features -- -D warnings

echo "3. Running contract unit and invariant test suite..."
cargo test --workspace

echo ""
echo "======================================================"
echo "All quality and verification checks passed."
echo "======================================================"
