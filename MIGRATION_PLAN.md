# Asset L2 Migration & Testnet Plan

This document replaces the previous migration notes. It captures the decisions from recent discussion and describes how to bring Asset L2 to a HotShot-based testnet with $ASSET as the native token.

## Goals
- **Pure Rust stack.** All on‑chain and off‑chain components are written in Rust.
- **HotShot consensus.** Use Espresso's HotShot implementation for BFT.
- **Automated testing.** Adopt HotShot's own `hotshot-testing` crate and `cargo nextest` for network tests.
- **Launch a public testnet** running the sequencer and Anchor program.

## 1. On‑chain program
- Build the batch verification contract using Anchor and `solana-program`.
- Provide serialization via Borsh. The existing `asset_rollup_program` crate is the foundation.
- Deploy to Solana devnet first, then upgrade to testnet.

## 2. Sequencer & BFT
- Implement networking with `tokio` and `libp2p`.
- Integrate HotShot as the consensus engine. The `hotshot` crate in this repo is a starting point.
- The sequencer posts state roots to the on‑chain program every block (≈250 ms).
- Run a committee of five‑to‑seven validators for the testnet.

## 3. Testing approach
- Add `hotshot-testing` as a dev‑dependency.
- Use `cargo nextest` to run both unit and integration tests. Mirror HotShot’s `just` tasks if desired.
- Write integration tests that spin up multiple HotShot nodes via `TestBuilder` and verify deterministic state roots and block commits.
- Keep existing unit tests to ensure VM and compiler correctness.

## 4. Continuous Integration
- Run `cargo fmt -- --check` and `cargo clippy -- -D warnings` in CI.
- Use `cargo nextest` in CI to execute the full test suite.
- Include a license file and set `license = "MIT"` (or chosen license) in each `Cargo.toml`.

## 5. Launching a testnet
1. **Build** all crates with `cargo build --workspace`.
2. **Run tests** with `cargo nextest run --workspace` to ensure both unit and integration tests pass.
3. **Deploy** the Anchor program to Solana devnet using `anchor deploy`.
4. **Start the sequencer**: launch several HotShot nodes that connect via `libp2p` and post batches through `BatchPoster`.
5. **Mint $ASSET** using the `assetvm` logic and exercise transfers via CurveVM programs.
6. **Monitor** state roots and validator performance. Iterate until stable, then open the network to external testers.

Following this plan brings Asset L2 from the current prototype to a fully tested HotShot‑backed testnet running the $ASSET token.
