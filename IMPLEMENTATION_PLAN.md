# Asset L2 Implementation Plan (Technical)

This plan translates `MIGRATION_PLAN.md` into concrete engineering steps with technical detail, interfaces, and deliverables. It is organized by workstream and includes dependencies, artifacts, and validation steps.

## 1) On-chain program (`asset_rollup_program`)
**Objective:** Anchor program that records batch roots, enforces authority, and provides upgrade/rollback safety.

### 1.1 Program state and accounts
- **Accounts**
  - `RollupConfig`:
    - `upgrade_authority: Pubkey`
    - `batch_poster_authority: Pubkey`
    - `paused: bool`
    - `version: u64`
    - `last_batch_height: u64`
  - `RootRegistry`:
    - `entries: Vec<RootEntry>` (or paging via PDAs)
    - `RootEntry { height: u64, root: [u8; 32], slot: u64, timestamp: i64 }`
- **PDAs**
  - `rollup_config_pda = seeds([b"rollup-config"])`
  - `root_registry_pda = seeds([b"root-registry"])`
- **Serialization**
  - Borsh for all account structs and instruction args.

### 1.2 Instruction set
- `initialize(config: RollupConfigInit)`
  - Creates config + registry accounts.
  - Sets authorities and initial version.
- `submit_batch_root(args: SubmitRoot)`
  - Checks `!paused`.
  - Verifies `signer == batch_poster_authority`.
  - Enforces monotonic height: `args.height == last_batch_height + 1`.
  - Appends `RootEntry` and updates `last_batch_height`.
- `rotate_authority(new_authority: Pubkey)`
  - Only `upgrade_authority` can rotate.
- `pause_posting()` / `resume_posting()`
  - Only `upgrade_authority`.
- `set_version(new_version: u64)`
  - Audit trail for upgrades.

### 1.3 Safety & rollback
- Store `last_good_version` in config or in a separate `ProgramVersion` account.
- Maintain deployment registry metadata (program ID, IDL hash, artifact checksum).

### 1.4 Devnet deployment pipeline
- `anchor build` / `anchor deploy` with explicit cluster config.
- Store program ID/IDL in `testnet` crate config.

**Deliverables:**
- Complete Anchor program with tests for each instruction and error path.
- Deployment script and registry metadata.

---

## 2) Sequencer & HotShot integration (`sequencer`, `hotshot`)
**Objective:** HotShot-backed sequencer that commits batch roots every block.

### 2.1 Network layer
- **Stack**: `tokio` runtime + `libp2p` with mDNS/bootstrap support.
- **Protocols**
  - `gossip/proposal` for block proposals.
  - `gossip/commit` for committed blocks.
- **Config**
  - `p2p.listen_addr`, `p2p.bootstrap_peers`, `p2p.node_key`.

### 2.2 HotShot wrapper
- Implement `HotShotNode` interface in `hotshot` crate:
  - `start()`, `stop()`, `submit_transaction(tx)`, `on_commit(handler)`.
- Define `BlockPayload` schema:
  - `height: u64`
  - `state_root: [u8; 32]`
  - `txs: Vec<Tx>` (optional, depending on sequencing design)
- Implement consensus integration with state machine updates and deterministic root computation.

### 2.3 Sequencer pipeline
- **Pipeline stages**
  1. Ingest txs or scripts.
  2. Compile (if needed) → `curvevm`/`assetvm` instructions.
  3. Execute VM → produce state root.
  4. Package root into `BlockPayload`.
  5. Submit to HotShot.
- **BatchPoster**
  - On commit, call `submit_batch_root` via Anchor client.
  - Retain retry queue with exponential backoff.
  - Persist last committed height locally.

### 2.4 Timing and committee
- **Block cadence**: target 250ms (configurable).
- **Committee**: 5–7 validators with config-driven membership and key rotation.

**Deliverables:**
- Running sequencer node with HotShot consensus and on-chain posting.
- Config files and example local multi-node setup.

---

## 3) Testing & determinism (`hotshot-testing`, workspace tests)
**Objective:** Deterministic state roots and network integration tests.

### 3.1 Dependencies and tooling
- Add `hotshot-testing` as `dev-dependency` where needed.
- Configure `cargo nextest` for workspace.

### 3.2 Integration test scenarios
- **TestBuilder**-based test harness:
  - Start N HotShot nodes with controlled network.
  - Submit transactions across nodes.
  - Assert consistent root across validators.
- **Reorg and fault tests**
  - Inject network delay / dropped messages.
  - Validate max reorg window ≤ 2 blocks.
- **On-chain posting test**
  - Mock Anchor client or use devnet + localnet to verify root submission order.

### 3.3 Unit tests
- VM/Compiler tests for determinism and edge cases.
- Serialization round-trip tests for all message structs.

**Deliverables:**
- Deterministic integration tests and unit coverage across crates.

---

## 4) CI & compliance
**Objective:** Enforce formatting, linting, and test coverage in CI.

### 4.1 CI pipeline steps
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo nextest run --workspace`

### 4.2 License compliance
- Add `LICENSE` file.
- Add `license = "MIT"` in every `Cargo.toml`.

**Deliverables:**
- CI workflows updated and passing.

---

## 5) Testnet launch workflow (`testnet` crate)
**Objective:** Runnable workflow for public testnet.

### 5.1 Build & run
- `cargo build --workspace`
- `cargo nextest run --workspace`
- `anchor deploy` to devnet/testnet

### 5.2 Testnet runner
- `cargo run -p testnet` spins:
  - N HotShot validators.
  - Sequencer with BatchPoster.
  - Sample token mint + transfer (via `assetvm` / `curvevm`).

**Deliverables:**
- Documented end-to-end testnet runbook.

---

## 6) Monitoring & observability
**Objective:** Metrics, logs, and traces aligned to SLOs.

### 6.1 Metrics
- **Batch latency**: time from tx ingest to on-chain root publication.
- **Root divergence**: local root vs on-chain root comparisons.
- **Proof verification time**: on-chain and local verification timings.
- **Node health**: peer count, consensus timeout rate, CPU/RAM, mempool depth.

### 6.2 Logging & tracing
- Structured logs with `batch_id`, `root_hash`, `round_id`.
- Tracing spans for:
  - Batch construction
  - HotShot proposal/commit
  - Anchor submit

### 6.3 Dashboards
- Export metrics to `testnet` tooling dashboards.
- SLO graphs for:
  - p50/p95 block interval
  - TPS
  - reorg window

**Deliverables:**
- Observability stack for testnet operations.

---

## 7) Operational safety & incident response
**Objective:** Secure upgrades, key rotation, and incident response readiness.

### 7.1 Upgrade flow
- Anchor upgrade authority with staged rollout:
  - Devnet → Testnet
- Version bump recorded in program state.

### 7.2 Rollback
- Keep last-known-good program binary/IDL.
- Redeploy same program ID with prior binary.

### 7.3 Key rotation
- Scheduled rotation cadence (monthly) + emergency rotation process.
- Update committee metadata and restart validators.

### 7.4 Incident response
- Halt posting → rotate/revoke keys → redeploy if needed → audit → resume.

**Deliverables:**
- Operational playbooks and checklists.

---

## Dependencies and sequencing
1. **Parallel:** On-chain program (1), HotShot integration (2), Test harness setup (3.1).
2. **Then:** Integration tests (3.2), CI updates (4), Observability (6).
3. **Finally:** Testnet workflow (5), Operational safety (7).

---

## Validation checklist
- On-chain unit/integration tests pass.
- Multi-node HotShot determinism tests pass.
- Sequencer posts correct root heights.
- Metrics and logging visible in dashboards.
- CI gates on fmt/clippy/nextest.
