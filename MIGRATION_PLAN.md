# Asset L2 Rust Migration Plan

This document sketches a high level path to migrate the experimental Python prototype to production grade Rust tooling.  It focuses on two core pieces:

1. **Solana on-chain programs.**
2. **The off-chain sequencer and BFT consensus.**

## 1. Solana programs

- **Use the Anchor framework** (`anchor-lang`, `anchor-client`) for program development, testing and deployment. Anchor is widely adopted in the Solana ecosystem and provides audited primitives for PDAs, CPI calls and serialization.
- **Leverage the official `solana-program` crate** for low level runtime interfaces.
- Re-write CurveVM as a Rust crate that can compile to both native binaries (for off-chain simulation) and a Solana program module.
- For serialization of instructions and state roots, use the `borsh` or `anchor` provided macros.

## 2. Off-chain sequencer & BFT

- **Networking and async tasks** – base the service on `tokio` and `libp2p`. These crates are well maintained and battle tested in many blockchain projects.
- **Mempool** – adopt persistent data structures such as `dashmap` for concurrent access, optionally backed by `sled` or `rocksdb` for disk durability.
- **BFT consensus** – evaluate existing Rust implementations:
  - [HotShot](https://github.com/EspressoSystems/espresso-sequencer) – an open source HotStuff-based BFT engine used by Espresso. It is actively developed and packaged as reusable crates.
  - [CometBFT](https://github.com/cometbft/cometbft) – a Go implementation of Tendermint; bindings exist but would introduce a cross-language dependency.
  Given the desire for a pure Rust stack, HotShot is the natural starting point.
- **Transaction posting** – use the `solana-sdk` and `solana-client` crates to submit batch roots to the AssetRollup program on Solana.

## 3. Rollup frameworks

There is not yet a widely adopted "Solana rollup SDK." The ecosystem is evolving and several projects are building generic rollup toolkits:

- [Rollkit](https://github.com/rollkit/rollkit) – a sovereign rollup framework in Go used with Celestia for data availability.
- [Sovereign SDK](https://github.com/sovereign-labs/sovereign-sdk) – a Rust-based toolkit for optimistic and zk rollups that is still under heavy development.

While these frameworks are not Solana-specific, they offer reusable components (DA layers, state trees, proof systems) that could be adapted. Anchor and the Solana runtime remain the most production tested choice for writing programs that interact with Solana today.

## 4. Migration steps

1. **Define Rust crates** for `curvevm`, `compiler`, `sequencer`, and `solana-program` modules.
2. **Port tests** from `pytest` to Rust's `cargo test` framework.
3. **Integrate HotShot** as the consensus engine and connect it to the Rust mempool and miner logic.
4. **Use Anchor** to implement the on-chain batch verification program.
5. **Automate builds** with `cargo` and set up CI (e.g., GitHub Actions) to run `cargo fmt`, `clippy`, and tests.

This phased approach retains the current Python prototype as a specification while the Rust rewrite matures. Once feature parity is reached, deprecate the Python modules and rely solely on the Rust crates for production.

## Migration progress

The repository now contains Rust crates for the CurveVM, compiler, sequencer, HotShot-based consensus and an Anchor rollup program.  Unit tests cover the mempool, miner and consensus logic.  A GitHub Actions workflow runs `cargo test` for CI.
