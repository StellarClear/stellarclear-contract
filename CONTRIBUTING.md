# Contributing to stellarclear-contract

## Quick start

```bash
rustup target add wasm32v1-none
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
stellar contract build
```

Toolchain: Rust `1.84.0+`, Stellar CLI `28.1.0+`, Soroban SDK `27.0.4`.

## How to pick up an issue

- Look for `good first issue` labels, comment to request assignment.
- One issue per PR, add/extend tests in `contracts/settlement-registry/src/test.rs`.
- State-machine changes must include invariant tests (no illegal transitions, finalized is terminal).
- Run full `cargo test --workspace` + fmt + clippy before pushing.

All contributions under Apache-2.0.
