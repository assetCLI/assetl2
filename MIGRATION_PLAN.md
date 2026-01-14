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
5. **Mint $ASSET** using the `assetvm` logic and exercise transfers via CurveVM programs. A sample workflow lives in the `testnet` crate and can be run with `cargo run -p testnet`.
6. **Monitor** state roots and validator performance. Iterate until stable, then open the network to external testers.

## 6. Monitoring & SLOs
### Required metrics
- **Batch latency** (time from sequencer ingest to root publication). Emit from the **sequencer** and surface in the **testnet** crate dashboards.
- **Root divergence** (difference between expected local root and on-chain committed root). Emit from **sequencer** checks and verify against the **rollup program** on-chain state in **testnet**.
- **Proof verification time** (elapsed time for on-chain verification and local verification). Emit from the **rollup program** and from local **testnet** harness verification.
- **Node health** (peer count, consensus round timeouts, mempool depth, CPU/RAM). Emit from the **sequencer** and aggregate in **testnet**.

### Logging & tracing requirements
- Structured logs with request IDs for batch IDs, root hashes, and consensus rounds.
- Trace spans for batch construction, HotShot proposal/commit, and on-chain submission.
- Export logs and traces from **sequencer** services; emit verification logs from the **rollup program**; consolidate in **testnet** observability tooling.

### Initial SLO targets
- **Block interval:** p50 ≤ 250 ms, p95 ≤ 500 ms.
- **Max acceptable reorg window:** ≤ 2 blocks.
- **Target TPS:** 250–500 TPS sustained during testnet load runs.
- **Root divergence:** 0 tolerated mismatches between local and on-chain roots.

## 7. Operational Safety
### Anchor program upgrade & rollback
- **Upgrade path:** use Anchor program upgrades with a dedicated upgrade authority, only after a staged devnet → testnet rollout and a signed release checklist.
- **Rollback path:** keep the last-known-good program binary and IDL in the deployment registry; if an upgrade regresses, immediately redeploy the prior binary with the same program ID and verify state compatibility.
- **Sequencer coordination:** pause batch posting during the upgrade window, then resume only after post-deploy health checks and a confirmed on-chain version hash.

### Validator key rotation
- **Planned rotation:** rotate validator identity keys on a scheduled cadence (e.g., monthly) with a coordinated epoch boundary and updated committee metadata.
- **Unplanned rotation:** if key material exposure is suspected, rotate immediately and remove the old key from the committee and any allowlists.
- **Operational steps:** publish new keys to the config registry, restart validators with updated keystores, and run a short canary round to confirm consensus participation.

### Compromised key handling
- **Immediate containment:** halt batch posting, revoke sequencer/BatchPoster credentials, and remove compromised validator keys from the committee.
- **On-chain response:** redeploy the Anchor program if the upgrade authority is compromised; rotate program upgrade authority and reissue PDA seeds if needed.
- **Recovery:** generate new keys offline, update the committee set, and resume posting only after integrity checks and audit sign-off.

### Incident response checklist (minimal)
1. **Halt posting** (stop sequencer and BatchPoster submissions).
2. **Invalidate keys** (revoke compromised validator and BatchPoster keys).
3. **Redeploy if required** (new upgrade authority and new PDA seed where applicable).
4. **Rotate + audit** (issue new keys, update committee, verify program state).
5. **Resume** only after on-chain checks and monitoring confirm stability.

Following this plan brings Asset L2 from the current prototype to a fully tested HotShot‑backed testnet running the $ASSET token.
