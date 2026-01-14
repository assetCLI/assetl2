# Execution Plan for Sharded Perp Integration

## Overview
The current codebase provides minimal stubs for the AssetScript DSL, compiler, CurveVM, sequencer, and rollup program. For example, AssetScript only parses `MINT`/`TRANSFER`/`BURN` commands【F:assetscript/src/lib.rs†L1-L60】, CurveVM exposes a four-opcode bonding-curve toy【F:curvevm/src/lib.rs†L1-L64】, and the Anchor rollup program merely stores a hash without proof validation【F:asset_rollup_program/src/lib.rs†L1-L51】. The sharded perp design document expects rich Router/Slab/Oracle intents, proof-carrying bytecode, capability-scoped syscalls, and HotShot orchestration【F:SHARDED_PERP_CURVE_INTEGRATION.md†L40-L300】. The plan below bridges that delta.

## Phase 1 – Extend CurveScript & Intent Surfaces
**Deliverables**
- Parser + schema support for `ROUTER`, `SLAB`, and `ORACLE` blocks, capability descriptors, and deterministic ID helpers as specified in §1 and §4【F:SHARDED_PERP_CURVE_INTEGRATION.md†L40-L166】.
- Validation logic that enforces cross-references (oracle IDs, batch tolerances) and governance guardrails (fee caps, latency SLAs).
- Manifest emission (structured JSON/Borsh) capturing CPI interface descriptors and capability schemas for downstream tooling (§5.8)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L260-L313】.

**Prerequisites**
- None (foundational phase).

**Exit criteria**
- Parser accepts a representative Router/Slab/Oracle spec and emits a manifest schema v1 with deterministic IDs.
- Validation tests cover cross-reference failures and governance guardrails with >90% branch coverage for the validation module.

## Phase 2 – Proofed Compiler & Manifest Pipeline
**Deliverables**
- New compiler pipeline in `compiler/` that lowers expanded CurveScript intents into Router + slab CurveVM modules, replacing the current direct opcode mapping【F:compiler/src/lib.rs†L1-L53】.
- Proof engine crate that generates R1CS/PLONK proofs for safe debit, reservation bounds, and margin math invariants (§§2–5, §6)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L65-L300】【F:SHARDED_PERP_CURVE_INTEGRATION.md†L326-L357】.
- Deployment manifests bundling bytecode hashes, proof hashes, config fingerprints, memory layout metadata, and CPI descriptors for rollup verification (§4, §6, §5.8)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L176-L206】【F:SHARDED_PERP_CURVE_INTEGRATION.md†L300-L313】.
- Static analyzers for slab memory budgets and capability scopes before code generation (§3.1, §4)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L120-L149】【F:SHARDED_PERP_CURVE_INTEGRATION.md†L176-L206】.

**Prerequisites**
- Phase 1 manifest schema v1 finalized and used by the compiler pipeline.

**Exit criteria**
- Compiler emits manifest schema v1 with hash coverage tests passing for bytecode/proof/config fingerprints.
- Proof engine generates proofs for the reference margin math circuits with deterministic public inputs and >=95% unit test coverage on proof serialization.
- Static analyzers reject over-budget slab layouts with golden tests for at least three failure cases.
### Phase 2 Proof System & Verification Boundaries
- **Proof system family**: target PLONKish systems (e.g., PLONK or Halo2) for universal circuits; keep a fallback path for R1CS/Groth16 if proving latency is critical for early pilots.
- **Trusted setup**: prefer transparent/no-trusted-setup (Halo2/KZG with ceremony optional); if Groth16 is selected for early deployments, document the circuit-specific trusted setup and rotate it per program upgrade.
- **On-chain vs off-chain verification**: off-chain (compiler/prover) builds proofs for margin math, reservation bounds, cap enforcement, and safe debit/credit accounting; on-chain (rollup program) verifies proof validity against the manifest and state root commitments, rejecting any proof that does not match the declared program hash.
- **Invariants & binding**: proofs must show margin math correctness, cap enforcement, reservation constraints, and deterministic fee/price application; proofs are bound to bytecode/manifest hashes via public inputs that include the Router/slab bytecode hash, descriptor hash, and config fingerprint so the verifier enforces code+config integrity.

## Phase 3 – CurveVM Runtime Upgrades
**Deliverables**
- Capability-scoped syscalls (escrow reads/writes, cap mint/burn, clock access), deterministic metering, hashing/Merkle precompiles, and constant-time u128 arithmetic (§2.2–§2.4, §4 CurveVM bullet, §5.2)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L72-L115】【F:SHARDED_PERP_CURVE_INTEGRATION.md†L187-L244】.
- Linear-memory management for 10 MB slab segments, metadata overlays, and pointer-safe execution for matching/reservation loops (§3)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L118-L149】.
- Telemetry hooks and gas accounting surfaced to the sequencer for scheduling (§2.2, §5.3)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L72-L115】【F:SHARDED_PERP_CURVE_INTEGRATION.md†L247-L261】.

**Prerequisites**
- Phase 2 compiler emits syscall descriptors and capability metadata in the manifest.

**Exit criteria**
- CurveVM syscall coverage >= 90% across Router/Slab golden programs.
- Deterministic metering tests pass with identical gas totals across 100+ randomized runs.
- Memory safety tests confirm no out-of-bounds access across slab stress fixtures.

## Phase 4 – Router & Slab Runtime Modules
**Deliverables**
- Router modules that implement reserve, commit, liquidation, registry update, funding, and telemetry entrypoints using the CurveVM extensions (§2.2–§2.4, §5.3–§5.6)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L72-L299】.
- Slab modules with deterministic matching, reservation accounting, anti-toxicity controls, funding loops, and liquidation handlers (§3, §5.4–§5.6)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L118-L299】.
- Program families consume capability manifests and CPI descriptors during compilation and expose hooks for the sequencer (batch windows, TTL enforcement) (§5.2, §5.4, §5.9)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L235-L359】.

**Prerequisites**
- Phase 2 compiler output + Phase 3 CurveVM syscalls available in integration tests.

**Exit criteria**
- Router + slab modules compile from a canonical CurveScript spec with deterministic bytecode hashes.
- Deterministic matching regression suite passes with seeded order books and reservation flows.
- CPI descriptor validation passes with zero missing capability checks.

## Phase 5 – Rollup & Anchor Integration
**Deliverables**
- `asset_rollup_program/` stores and verifies (bytecode_hash, proof_hash, descriptor_hash) tuples, enforces capability scopes, and gates vault writes on proof validation (§2.4, §5.1, §6)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L96-L244】【F:SHARDED_PERP_CURVE_INTEGRATION.md†L326-L336】.
- Rollup state management (new crate or extend `assetvm/`) tracks escrow PDAs, caps, portfolio accounts, reservation queues, and governance registries as described in §2.1 and §5.1【F:SHARDED_PERP_CURVE_INTEGRATION.md†L65-L244】.
- Proof verification caching and Merkle path auditing in the HotShot executor / state tree layer (§4 AssetL2 Rollup, §6)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L197-L206】【F:SHARDED_PERP_CURVE_INTEGRATION.md†L326-L336】.

**Prerequisites**
- Phase 2 manifests finalized with proof/hash fields.
- Phase 4 Router/slab modules output stable bytecode hashes for verification.

**Exit criteria**
- Rollup verifier accepts valid manifests and rejects mismatched hashes in unit tests.
- Proof verification bench under 50 ms per proof on the target devnet hardware profile.
- State tree audits detect malformed Merkle paths in fuzz tests with 0 false negatives.
### Phase 5 Proof Verification & Trust Model
- **Proof system family**: verify PLONKish proofs on-chain when practical (batch verification if supported), with a fallback path to Groth16 if on-chain verifier costs dominate; keep the verifier interface abstracted by proof hash + verifier ID in the manifest.
- **Trusted setup**: if Groth16 is used, store/verifiable reference to the ceremony transcript hash in the rollup configuration; transparent schemes (e.g., Halo2) require no trusted setup.
- **On-chain vs off-chain verification**: off-chain sequencer/prover aggregates proofs and supplies manifests + public inputs; on-chain `asset_rollup_program` verifies proofs, enforces that public inputs match bytecode/manifest hashes, and only then updates roots or vault state.
- **Invariants & binding**: on-chain verification checks that proofs attest to margin math, cap enforcement, reservation accounting, and fee correctness, and that the proof public inputs commit to the bytecode hash, descriptor hash, and config fingerprint so upgrades cannot bypass safety checks.

## Phase 6 – Sequencer & HotShot Orchestration
**Deliverables**
- HotShot-driven batching that understands reserve→commit workflows, TTL enforcement, and descriptor hashes (§2.4, §4 AssetL2 Rollup, §5.4, §5.9, §7)【F:sequencer/src/lib.rs†L1-L204】【F:SHARDED_PERP_CURVE_INTEGRATION.md†L110-L359】【F:SHARDED_PERP_CURVE_INTEGRATION.md†L340-L359】.
- Proof manifest verification before BatchPoster commits and metadata included with Solana submissions (§4 AssetL2 Rollup, §5.8)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L197-L313】.
- Failure-mode handling (cap expiry cancels, state root divergence detection, network partition recovery) in line with §5.9 and §13 acceptance criteria【F:SHARDED_PERP_CURVE_INTEGRATION.md†L340-L402】【F:SHARDED_PERP_CURVE_INTEGRATION.md†L430-L452】.

**Prerequisites**
- Phase 5 rollup verification interfaces available and stable.

**Exit criteria**
- End-to-end batching test covers reserve→commit→liquidation with proof checks and succeeds under load.
- Sequencer detects state root divergence in chaos tests within one batch interval.
- Network partition recovery test replays queued batches without data loss.

## Phase 7 – Oracle Adapters, SDKs & Ops Tooling
**Deliverables**
- Oracle ingestion modules shared by Router and slabs, plus failover configuration, matching §9 deliverables【F:SHARDED_PERP_CURVE_INTEGRATION.md†L360-L399】.
- Auto-generated CPI bindings and client SDK guardrails from the CurveScript descriptors (Rust/TypeScript crates) per §5.8 and §9【F:SHARDED_PERP_CURVE_INTEGRATION.md†L300-L313】【F:SHARDED_PERP_CURVE_INTEGRATION.md†L360-L399】.
- Operational runbooks, telemetry exporters, and benchmarking harnesses demanded in §9–§13 to satisfy go/no-go checks【F:SHARDED_PERP_CURVE_INTEGRATION.md†L360-L452】.

**Prerequisites**
- Phase 4 Router/slab module interfaces finalized for adapter bindings.
- Phase 6 sequencer emits telemetry hooks for ops tooling.

**Exit criteria**
- Oracle adapter integration tests pass with failover switching within 2 seconds.
- SDKs generated for Rust + TypeScript with compile-time checks against manifest schema v1.
- Ops benchmarks produce dashboards with p95 latency and throughput thresholds defined.

## Phase 8 – Testing & CI Enablement
**Deliverables**
- Expanded unit/property tests across `assetscript`, `compiler`, `curvevm`, `sequencer`, and the rollup to cover USP-aware reserve→commit→liquidation flows, proof verification, and failure scenarios (§5.7, §5.9, §8)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L314-L359】【F:SHARDED_PERP_CURVE_INTEGRATION.md†L360-L399】.
- Integration tests that spin HotShot nodes, execute perp programs, and validate cross-app collateral accounting alongside bonding-curve workflows (§5.3, §8, §13)【F:SHARDED_PERP_CURVE_INTEGRATION.md†L247-L452】.
- CI updates to run `cargo fmt`, `cargo clippy -D warnings`, `cargo nextest`, proof regression checks, and long-running soak tests, aligning with MIGRATION_PLAN.md guidance.【F:MIGRATION_PLAN.md†L1-L43】

**Prerequisites**
- Phase 6 sequencing workflows stable and Phase 7 SDKs available for integration tests.

**Exit criteria**
- CI runs full pipeline (fmt/clippy/nextest/proof regression) on main branch with green status for 10 consecutive runs.
- HotShot integration suite passes with >=90% coverage on cross-app collateral flows.
- Soak tests complete a 24h run with no memory leaks above 5% RSS drift.

## Cross-phase dependencies
- Manifest schema v1 (Phase 1) must be finalized before compiler proofs (Phase 2) and rollup verification (Phase 5) can lock public input formats.
- CurveVM syscall surface (Phase 3) must land before Router/slab module generation (Phase 4) can reach feature parity.
- Router/slab deterministic hashes (Phase 4) must be stable before rollup verification and sequencer checks (Phases 5–6).
- Rollup verifier interfaces (Phase 5) must be stable before sequencer batching and SDK generation (Phases 6–7).
- Sequencer telemetry exports (Phase 6) must exist before ops tooling and CI dashboards (Phases 7–8).
