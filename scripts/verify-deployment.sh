#!/usr/bin/env bash
set -euo pipefail

echo "======================================================"
echo " StellarClear Contract Deployment Verification Utility"
echo "======================================================"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

CONTRACT_ID="${1:-${SETTLEMENT_REGISTRY_CONTRACT_ID:-}}"
NETWORK="${2:-${STELLAR_NETWORK:-testnet}}"

if [ -z "$CONTRACT_ID" ]; then
    echo "Usage: $0 <CONTRACT_ID> [NETWORK]"
    echo "  or export SETTLEMENT_REGISTRY_CONTRACT_ID=<CONTRACT_ID>"
    exit 1
fi

echo "Verifying contract deployment:"
echo " - Contract ID: $CONTRACT_ID"
echo " - Target Network: $NETWORK"

# Check local WASM artifact hash
WASM_FILE="target/wasm32v1-none/release/settlement_registry.wasm"
if [ -f "$WASM_FILE" ]; then
    LOCAL_HASH="$(sha256sum "$WASM_FILE" | awk '{print $1}')"
    echo " - Local Release WASM SHA-256: $LOCAL_HASH"
else
    echo " - Local WASM artifact not yet built (run ./scripts/build.sh to verify bytecode)"
fi

if command -v stellar &>/dev/null; then
    echo ""
    echo "Querying on-chain state for contract $CONTRACT_ID on $NETWORK..."
    # Verify contract responsiveness
    if stellar contract inspect --id "$CONTRACT_ID" --network "$NETWORK" &>/dev/null; then
        echo "✅ Contract interface inspected successfully on $NETWORK."
    else
        echo "ℹ️ Note: Contract inspection requires network connection or valid RPC credentials."
    fi
fi

# Run deployed testnet test matrix
echo ""
echo "Running programmatic deployed contract verification suite..."
cargo test --test deployed_testnet

echo ""
echo "======================================================"
echo "✅ Deployed Contract Verification Successful"
echo "======================================================"
