# Agent Reflection

## Overall Goal
Asset L2 is an application-specific roll-up anchored to Solana. The end goal is to own the entire token-launch pipeline so creators can describe a bonding curve in plain English and receive safe, provably correct on-chain code. The design ties together a minimal VM, a HotStuff‑2 sequencer and a compiler that proves generated code. The project mirrors the vision laid out in `assetl2.md`.

## Code Structure
The repository is organised as a Rust workspace with several crates:

- **curvevm** – four-opcode WASM runtime executing buy/sell and liquidity ops.
- **compiler** – parses CurveScript and emits `curvevm` instructions.
- **assetvm** – simplified asset ledger used by the script layer.
- **assetscript** – toy DSL for asset instructions mirroring CurveScript style.
- **sequencer** – basic HotShot-based miner posting batches via `BatchPoster`.
- **asset_rollup_program** – Anchor program that records batch roots on Solana.
- **hotshot** – stub of the HotShot consensus engine used by the sequencer.

Each crate exposes a `lib.rs` with unit tests.  `Cargo.toml` at the workspace root lists them all.

## Coding Approach
We aim for a pure Rust stack.  Crates use Borsh for serialization and depend on `anchor-lang` and `solana-program` for on-chain pieces.  Edition 2024 is enabled to experiment with upcoming Rust features.  The normal flow is:

1. Parse a script to commands (`assetscript` or `compiler`).
2. Compile commands into VM instructions.
3. Execute instructions in `curvevm` or `assetvm` to compute a state root.
4. The sequencer packages instructions into blocks and posts a Merkle root through `asset_rollup_program`.

Every crate includes minimal unit tests and the workspace compiles with `cargo test`.

## Mistakes & Lessons
- Initially left Python code and tests in the repo, conflicting with the pure Rust goal. These were removed.
- Missed nested documentation at first. Later confirmed no additional `AGENTS.md` existed.
- Using Rust nightly features like edition 2024 causes build issues on stable toolchains, so the environment now installs a nightly toolchain via `rustup` to run tests.
- Early commits lacked a full description of module roles; this file addresses that.
- The GitHub workflow still ran Python tests even after dropping the code. We removed the Python job and added HotShot tests and a `testnet` crate to keep CI purely Rust.

## Future Work
- Flesh out the HotShot consensus implementation and integrate real networking per `MIGRATION_PLAN.md`.
- Replace the stubbed VM and compiler with the CurveScript/CurveGPT pipeline from the design doc.
- Add CI to run `cargo fmt`, `cargo clippy -D warnings` and `cargo nextest`.
- Keep the migration plan and design documents in sync with code progress.

