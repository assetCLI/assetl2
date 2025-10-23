# Sharded Perp DEX within the AssetL2 Stack

This document adapts the sharded perpetual exchange architecture to AssetL2's unique
CurveScript → CurveGPT → Compiler+Proof Engine → CurveVM → Rollup pipeline. The goal is
to preserve every router/slab invariant from the base design while making the system
provably compatible with AI-authored curves, micro-VM execution, and HotShot-based
sequencing. Engineers can implement the full protocol knowing how the USP layers own the
code-generation surface, enforce proofs, and provide deterministic ordering.

---

## 0. Stack Integration Map

| Layer | Role in Base Design | AssetL2 Adaptation |
| --- | --- | --- |
| CurveGPT + CurveScript | N/A | Generates domain-specific intents describing slab risk knobs, maker policies, and router allocation rules. Agents emit scripts in the CurveScript DSL rather than direct Rust. |
| Compiler + Proof Engine | Router/Slab Rust | Compiles scripts into CurveVM bytecode and attaches overflow/escrow safety proofs. Emits two artifacts per upgrade: (1) executable CurveVM programs for Router + slabs, (2) a proof transcript consumed by the rollup verifier. |
| CurveVM | Not present | Executes Router/slab bytecode inside the rollup. All state transitions run as CurveVM programs with deterministic resource bounds (<300k CU). |
| AssetL2 Rollup | Chain runtime | Hosts Router + slabs as rollup applications. HotShot consensus orders batches, then BatchPoster anchors Merkle roots on Solana. |
| Anchor program | Custody boundary | Solana-side program stores the Router vaults/escrows as before, but the entrypoints accept CurveVM-verified state roots instead of opaque CPI calls. |

The Router and slab logic remain segmented, but each upgrade flows through the USP pipeline:
1. CurveGPT drafts scripts for Router policies, slab parameters, and default instrument templates.
2. The Compiler lowers scripts to CurveVM instructions and proves arithmetic/cap invariants.
3. The rollup deploys the new programs by publishing the bytecode + proof hash; the on-chain
   registry enforces matching hashes before accepting slab commits.

---

## 1. CurveScript Surface

We extend CurveScript with three intent namespaces. Each block compiles into dedicated CurveVM
functions.

```text
ROUTER {
    COLLATERAL asset=USDC vault_cap=50000000
    PORTFOLIO_MARGIN model="cross_alpha" correl_matrix="router::correlations::v1"
    CAP_TTL ms=120000
    RESERVATION_BATCH ms=50
}

SLAB "perp:SOL-PERP" {
    MAKER_CLASS DLP allowance=5000000
    MATCHING fifo=true pending_promotion=true
    FEE maker_bps=2 taker_bps=5 rebate_delay_ms=50
    RISK imr_bps=500 mmr_bps=350
    ANTI_TOXICITY kill_band_bps=75 jit_penalty=true arg_tax_bps=10
}

ORACLE "pyth:SOLUSD" {
    HEARTBEAT ms=500
    KILL_BAND_SYNC router_ref="ROUTER"
}
```

* Every keyword maps to bounded, type-safe arguments proven by the compiler.
* Scripts never embed raw arithmetic; the compiler expands them into table-driven constants used
  by CurveVM code.
* The proof engine emits R1CS certificates for invariants such as `cap.remaining >= 0` and
  `reserved_qty <= qty`.

---

## 2. Router Program in CurveVM

### 2.1 State Layout (unchanged semantics)
- **Vault**, **Escrow**, **Cap**, **Portfolio**, and **SlabRegistry** mirror the base design.
- CurveVM opcodes manipulate these accounts using fixed memory slots; addresses are deterministically
  derived PDAs stored in the rollup state tree.

### 2.2 Top-Level Components (Router boundary)
We still deploy every base-architecture component—just emitted by the proofed compiler and hosted
inside the rollup:

- **Router Aggregator:** CurveVM module that orchestrates `reserve → escrow funding → cap mint →
  commit/cancel` flows. It exposes entrypoints matching the base wire protocol so external clients
  can reuse the same sequencing diagrams.
- **Global Vaults:** Solana custody accounts owned by the Anchor rollup program. The rollup presents
  a `VAULT_WRITE` syscall to the Router VM so only proof-verified code can debit/credit balances,
  satisfying P1–P3.
- **Escrow PDAs:** Deterministic `(user, slab, asset)` records maintained in the rollup state tree.
  Router bytecode receives the PDA seed material as constants; commits can only mutate escrow via
  the `safe_debit` template that the proof engine verifies.
- **Cap Records:** Short-lived capability objects stored alongside escrow balances. Router programs
  mint/burn caps and update `remaining` after each commit; slabs receive the cap handle as part of
  the CPI message emitted by the CurveVM runtime.
- **Portfolio Accounts:** Aggregated per-user exposure tracking that the Router updates after each
  slab commit. The compiler generates helper routines to recompute IM/MM and assert
  `equity = cash + Σ pnl`, mirroring P8–P9.
- **Slab Registry:** Governance-controlled table keyed by slab hash. Registry mutations require a
  proof manifest and feed slab parameters back into CurveScript validation (P10–P11).
- **Liquidation Coordinator:** Router entrypoint that interfaces with slabs through CurveVM syscalls
  enforcing capability scopes. It uses the same `liquidation_call` semantics as the base design and
  can re-pledge collateral before invoking slabs.
- **Telemetry Hooks:** CurveVM log events mirrored to rollup observability (reservations, kill-band
  rejections, cap expiries) so operations can audit the safety boundary in production.

These artifacts are generated alongside slab bytecode. The rollup deploys them as a cohesive
package so we keep the strict Router ↔ slab security boundary while still leveraging the AssetL2 USP
stack for authoring and verification.

### 2.3 Execution Model
- Each Router function (reserve orchestration, commit orchestration, liquidation, registry update)
  is a CurveVM entrypoint defined by the compiled script.
- CurveVM enforces deterministic gas: Router programs run in <200k CU thanks to four-opcode design.
- Safe debit logic is emitted as canonical template code. The proof engine supplies a proof artifact
  checked by the rollup before deployment; runtime checks remain to satisfy ADV1/P6/P7.

### 2.4 Integration with Rollup
- HotShot batches include Router VM invocations referencing caps/reserves created earlier in the
  same batch. Sequencer ordering ensures commit/cancel determinism without Solana race conditions.
- BatchPoster publishes the CurveVM state root; the Anchor contract verifies the root and permits
  vault updates from Router commits only when proofs are valid.

---

## 3. Slab Programs inside CurveVM

### 3.1 Memory Budget
- The 10 MB slab state is realized as a CurveVM linear memory segment. The compiler allocates fixed
  offsets for Header, Instruments, Orders, Reservations, etc., preserving S1–S12 invariants.
- CurveVM's four opcodes (`LOAD`, `STORE`, `ADD`, `BRANCH`) expand into inline pointer math proven
  overflow-safe by the proof engine.

### 3.2 Matching Loop
- The reserve and commit functions translate to deterministic CurveVM loops identical to the
  pseudocode in §11. The compiler injects range guards so pointer arithmetic cannot escape the slab
  segment.
- Anti-toxicity features (kill band, ARG, JIT penalty) are parameterized via script constants and
  executed with constant-time table lookups in CurveVM to stay within compute limits.

### 3.3 Upgrades via CurveGPT
- LPs describe desired policy changes in English; CurveGPT produces updated `SLAB { ... }` blocks.
- Governance runs the proof pipeline; only after proofs succeed does the Router registry accept the
  new slab hash. This aligns version-hash enforcement (P10/P11) with our USP.

### 3.4 Slab State Compatibility
- The slab data structures—Header, Instruments, Orders, Reservations, Positions, Trades, Aggressor
  ledger—keep the exact layout and invariants from the base design. No fields are dropped or
  reordered, which lets auditors reuse existing validation tooling and makes on-chain proofs
  mechanically comparable to the original specification.
- CurveVM integration adds *metadata overlays* instead of structural changes: each pool gains a
  compile-time constant describing its byte-range, and the proof engine tracks freelist pointers to
  ensure memory safety. These overlays live in the compiler manifest, not in slab memory, so the
  runtime slab footprint stays at 10 MB.
- Optional extensions (e.g., telemetry counters for USP observability) are modeled as auxiliary
  arrays stored after the 10 MB segment. They are write-only from the slab and do not participate in
  matching or risk logic, preserving determinism and satisfying the base invariants S1–S12.

---

## 4. Component Changes Required

The sharded perp design touches every USP pillar. Engineers should budget the following concrete
changes before attempting an implementation sprint:

### CurveScript
- Add first-class `ROUTER`, `SLAB`, and `ORACLE` blocks with schema validation and rich enums for
  anti-toxicity knobs, liquidation policies, funding cadence, and escrow lifetimes.
- Introduce capability descriptors (e.g., `CAP` statements) that emit bounded u128 amounts and TTLs
  so the compiler can materialize per-(user, slab, asset) caps without handwritten constants.
- Extend type checking to cover cross-reference validation (e.g., slabs must reference existing
  oracle IDs; router batches must match slab batch lengths within ±10 ms tolerance).
- Provide deterministic ID derivation helpers (`route_id`, `hold_id`) to prevent humans from
  hardcoding numeric identifiers.

### CurveGPT
- Fine-tune prompt templates to map natural-language LP policies into the new CurveScript blocks,
  including guardrails that reject unsupported constructs (e.g., multi-asset portfolios per slab).
- Add self-review routines that verify generated scripts obey governance guardrails (fee caps,
  latency SLAs, approved oracle lists) before passing them to the compiler.
- Generate proof annotations alongside scripts (e.g., `@invariant reserved_qty <= qty`) to help the
  compiler synthesize constraints without manual wiring.

### Proofed Compiler
- Implement a perp-aware lowering pass that converts CurveScript directives into CurveVM control
  flow templates for reservation loops, cap minting, funding and liquidation operations.
- Extend the constraint system with gadgets for 128-bit arithmetic, bounded sums of reservations,
  and tick/lot alignment so proofs can cover every invariant listed in §2 and §3.
- Emit dual artifacts: (1) CurveVM bytecode; (2) succinct proofs and verification keys. Integrate a
  manifest (bytecode hash, proof hash, config fingerprint) consumed by the rollup deployment
  pipeline.
- Provide a static analyzer that ensures slab memory layouts fit within 10 MB and flags scripts that
  would exceed freelist capacities before code generation.

### CurveVM
- Add host calls for reading/writing Router escrow accounts, minting/burning caps, and accessing the
  rollup clock, each limited by capability scopes validated at compile time.
- Extend the instruction interpreter with constant-time u128 arithmetic helpers to avoid overflow in
  notional and fee calculations while staying under the four-opcode budget via microcode sequences.
- Support deterministic syscall metering so Router + slab programs can run within 300k CU and expose
  per-invocation gas usage to the sequencer for scheduling.
- Provide precompiles for hashing (commit-reveal) and Merkle inclusion proofs used when slabs publish
  reservation commitments or state roots.

### AssetL2 Rollup
- Update the HotShot executor to accept CurveVM bytecode manifests, verify accompanying proofs, and
  cache verification keys for repeated use across batches.
- Extend the state tree to hold Router/slab account PDAs plus per-slab reservation queues, ensuring
  proofs cover Merkle paths when the Anchor program audits state updates.
- Enhance BatchPoster to bundle (bytecode_hash, proof_hash) metadata so Solana-side governance can
  reject unverified upgrades.
- Implement sequencer orchestration hooks that understand reserve→commit workflows: reserve
  transactions are queued within an epoch and the sequencer enforces commit ordering/expiry handling
  before finalizing the batch.

These upgrades guarantee that every layer—script generation, automated authoring, proofed
compilation, VM execution, and rollup governance—understands the Router/slab separation and the
capability boundaries described in the base architecture.

---

## 5. Protocol Mechanics within the USP

The base design's operational invariants continue to hold once routed through CurveScript, the
proofed compiler, CurveVM, and the rollup. The following subsections map each critical mechanic to
its supporting USP component so implementation teams can trace responsibility end to end.

### 5.1 Collateral & Security Boundaries
- **Vault Custody:** Anchor-owned Solana vaults accept writes only from Router CurveVM programs that
  arrive with valid proof hashes. BatchPoster updates include vault diff commitments, letting L1
  governance audit that no slab bytecode ever touches custody directly.
- **Escrow Isolation:** Escrow PDAs live in the rollup state tree. CurveVM exposes read-only access
  to slabs, while Router entrypoints receive write capabilities that the proof engine constrains to
  `(user, slab, asset)` tuples. The HotShot executor enforces that only Router transactions can carry
  the escrow-write syscall flag, satisfying P4–P7.
- **Cap Issuance:** Cap records are minted and burned by Router bytecode that the compiler emits
  from `CAP_TTL` and `CAP` directives. The resulting proof witnesses show that caps cannot be reused
  past expiry and cannot exceed escrow balances, extending ADV1 defenses into the USP stack.
- **Registry Governance:** CurveScript declarations include slab hashes and oracle bindings. The
  rollup verifier checks these against governance rules before allowing Router upgrades, keeping the
  Router↔slab trust boundary explicit even after AI-authored changes.

### 5.2 Capability-Scoped Debits & Reservations
- **safe_debit Template:** The proofed compiler instantiates a canonical `safe_debit` micro-program
  for every `(user, slab, asset)` combination. Both CurveVM bytecode and the accompanying proof
  ensure debits never exceed `cap.remaining` or escrow balance, covering P6–P7 and ADV1.
- **Reservation Tables:** Reservation loops compile into bounded CurveVM iterations that maintain
  `reserved_qty <= qty`. Proof gadgets record per-order deltas so concurrent reserves cannot
  over-allocate slices, preserving S8–S9 and M4–M5.
- **Commit-Phase Hashing:** CurveVM precompiles hash reservation payloads to enforce commit-reveal
  semantics. Sequencer logic refuses commits whose hashes do not match prior reserves, extending M6
  enforcement from L1 Solana ordering into the rollup batch domain.

### 5.3 Risk & Margining
- **Local Risk Checks:** CurveScript `RISK` directives define IM/MM coefficients. The compiler emits
  deterministic CurveVM routines that recompute slab equity and margin before and after each fill,
  flagging any violation via VM traps that revert the transaction. Proof circuits show arithmetic
  stays within 128-bit bounds, fulfilling RM1–RM3. Generated unit tests replay increasing exposure
  scenarios and closing trades to assert the monotonicity and zeroing behavior demanded by RM1 and
  RM2, while liquidation harnesses validate RM3’s tick-tolerant triggers.
- **Global Portfolio Margin:** Router `PORTFOLIO_MARGIN` blocks wire correlation matrices into
  CurveVM functions that aggregate exposures across slabs. Portfolio updates run after every commit
  and liquidation, with proofs ensuring `equity = cash + Σ pnl`, covering P8–P9 and RM4–RM5. CI
  regression suites explicitly compare ΣIM_slab against IM_router under hedged and directional
  positions so reviewers can confirm convexity penalties are applied exactly once.
- **Risk Telemetry:** CurveVM emits structured logs for IM/MM utilization that the rollup streams to
  monitoring. This mirrors existing Router invariants so operators can detect divergence between
  local and global margin engines and feeds economic simulations that check margin buffers against
  RM-series baselines.

### 5.4 Matching, Reservations & Anti-Toxicity
- **Deterministic Matching:** Matching loops are emitted as straight-line CurveVM code with explicit
  price-time comparisons. Proof annotations confirm order IDs remain monotonic, securing M1–M3 and
  S5–S7. Compiler-level property tests replay random post/cancel sequences and compare the resulting
  execution trace against a reference simulator to lock in FIFO behavior.
- **Reservation Accounting:** Reservation tables compile into bounded loops that maintain
  `reserved_qty <= qty` even under concurrent holds. Fuzzers spawn parallel reserves to enforce M4
  and M5, while descriptor-driven serialization ensures R1–R3 validation happens before the VM ever
  touches slab memory.
- **Commit-Reveal Discipline:** CurveVM exposes a hashing precompile for commitment payloads. The
  sequencer checks the digest on reveal, providing deterministic enforcement of M6–M7 while keeping
  latency predictable inside HotShot epochs.
- **Batch Windows & Pending Promotions:** `RESERVATION_BATCH` directives inform sequencer scheduling.
  HotShot batches tag reserves with epoch IDs; only commits within the same epoch and before TTL
  expiry are accepted, delivering the same anti-toxicity guarantees as the on-chain design. Pending
  order queues map to fixed memory offsets with promotion routines tied to epoch increments; CurveVM
  guards prevent double promotion, preserving R9 consistency.
- **Anti-Sandwich Controls:** Kill bands, JIT penalties, top-K freezes, and the optional Aggressor
  Roundtrip Guard compile into table-driven checks that run during commit. Generated tests replay
  overlapping aggressive legs and late maker postings to verify M8–M11 and ADV2/ADV3 behavior, while
  sequencer simulations ensure kill-band oracle checks stay deterministic across replicas.

### 5.5 Funding & Fee Settlement
- **Funding Grid Sharing:** CurveScript associates slabs with oracle IDs and funding cadence. The
  Router CurveVM module polls shared funding data, accrues transfers once per interval, and emits
  mirrored entries into portfolio accounts. Proofs show funding is applied exactly once (RM6–RM7),
  and integration tests replay mismatched oracle schedules to ensure double-application attempts
  revert.
- **Fee Caps:** Governance-approved fee caps compile into constant tables. During commit, CurveVM
  compares slab-reported fees against these caps before applying them, enforcing ADV3 within the VM
  runtime and giving auditors deterministic hooks to check fee drift alongside funding receipts.

### 5.6 Liquidation & Recovery
- **Router-First Remediation:** Router programs can reassign collateral across slabs using
  CurveVM syscalls that respect cap scopes. Proof witnesses show the router cannot exceed escrow
  balances while attempting grace-period remediation (L1–L2 coverage), and CI harnesses inject
  margin top-ups during the grace window to confirm slabs stand down when the router restores
  solvency.
- **Slab Sweeps:** Slab `liquidation_call` handlers run pre-proved matching loops limited by kill
  bands and cap balances. Sequencer orchestration ensures only Router-authored liquidation invocations
  execute, preventing user-triggered griefing while preserving L3 limits. Stress tests explore
  maximum-depth sweeps to prove caps never allow over-debit beyond escrow.
- **Insurance Access:** Any insurance debit requires governance-approved manifests. CurveVM code can
  only touch insurance vaults through explicit manifests checked by the Anchor program, mirroring the
  base design's isolation policy and letting auditors replay liquidation traces alongside insurance
  attestations.

### 5.7 Testing Responsibilities
- **USP-Aware Unit Tests:** Compiler and CurveVM crates must ship unit tests that replay reserve →
  commit → liquidation flows, asserting both arithmetic invariants and proof verification results.
- **Sequencer Integration Tests:** HotShot simulations cover epoch timing, kill-band rejections, and
  liquidation call ordering to ensure timing-sensitive invariants survive batching.
- **Cross-Layer Regression:** CI combines Router/slab VM execution with rollup proof verification to
  demonstrate that capability-scoped debits, funding accrual, and risk updates match base design
  expectations across upgrades.

### 5.8 Cross-Program Interfaces (CPI) Autogeneration
- **Interface Descriptors in CurveScript:** `ROUTER` and `SLAB` blocks emit declarative CPI schemas
  for `reserve`, `commit`, `cancel`, `batch_open`, and `liquidation_call`. Each schema specifies the
  account list, capability handles, and serialized argument layout (lot-aligned quantities, captured
  maker prices, kill-band settings). Because these descriptors are script-level artifacts, product
  teams never hand-author Anchor CPI glue; instead, governance reviews human-readable manifests
  before approving upgrades.
- **Compiler Output:** The proofed compiler consumes the descriptors and generates (1) CurveVM entry
  shims that marshal arguments into slab memory and (2) host-side client bindings (Rust/TypeScript)
  that enforce the same serialization order. Proof witnesses cover the shims, guaranteeing that
  CurveVM bytecode only touches accounts enumerated in the descriptor and that all CPI messages stay
  within declared bounds. This satisfies the base design's R1–R9 interface guarantees without
  bespoke plumbing.
- **Rollup & Solana Integration:** The AssetL2 rollup exports CPI manifests to the Anchor
  program—`BatchPoster` includes the descriptor hash alongside the bytecode/proof hashes—so Solana
  validators can confirm the Router is invoking slabs with the pre-approved account/argument layout.
  Sequencer logic cross-checks incoming transactions against the descriptors to ensure batch
  ordering, TTL enforcement, and capability scope (caps/escrows) stay aligned with the autogenerated
  interface.
- **Client SDK Guardrails:** Generated SDKs reject mismatched schemas at compile time (via type
  signatures) and at runtime (via descriptor hash checks), reducing integration risk for liquidity
  providers who interact with the Router directly. Developers can extend the SDKs without touching
  core CPI code because regeneration is deterministic given the CurveScript inputs.

---

### 5.9 Failure Handling & Liveness Guarantees

- **Cap Expiry Discipline:** `CAP_TTL` directives in CurveScript define short-lived capabilities
  (≤ 120 seconds). The proofed compiler emits timeout checks that the CurveVM runtime enforces on
  every `commit` entrypoint; once the TTL elapses, the VM traps before attempting any debit, keeping
  escrow balances untouched (F1). Sequencer logic watches the same TTL metadata and automatically
  queues `cancel` calls for outstanding holds so caps are burned and reservations released without
  manual intervention.
- **Router Crash Safety:** Because Router state lives in the rollup tree and vault custody is gated
  by the Anchor program, a stalled Router node cannot leak funds. HotShot consensus pauses batch
  finalization while CurveVM caps continue to expire naturally; once operators restart, the
  sequencer replays pending transactions against the same state root, guaranteeing idempotent
  `cancel` execution (F2) and preserving at-most-once semantics for commits.
- **Slab Liveness & Re-Routing:** Slabs run as independent CurveVM modules registered in the
  governance-controlled hash registry. If a slab misses its batch window, the sequencer consults the
  registry’s exposure ceilings (`E_max`) and automatically redistributes new reserves across healthy
  slabs while marking the stale holds for cancellation. The Router’s liquidation coordinator
  leverages the same capability-scoped syscalls to retry `commit` with fresh caps, ensuring the
  at-most-once property even across network partitions (F3).

- **Failure-Mode Test Hooks:**
  - *F1 – Cap Expiry:* VM unit tests simulate expired caps and assert that escrow balances and
    `cap.remaining` stay unchanged while proofs verify the trap condition.
  - *F2 – Idempotent Cancel:* Rollup integration tests repeatedly replay `cancel` transactions
    across sequencer restarts to demonstrate identical post-state roots.
  - *F3 – Network Partition:* HotShot simulations drop Router ↔ slab messages mid-batch and verify
    that once connectivity returns, only one of the retries can spend the cap nonce.

---

## 6. Proving & Verification Flow

1. **Compilation:** Scripts compile to CurveVM bytecode plus machine-checked invariants. Proofs cover:
   - Cap debits respect escrow balances (ADV1, P6–P7).
   - `reserved_qty` never exceeds order quantity (S8).
   - Price-time order comparisons use monotone IDs with overflow-free arithmetic.
2. **Rollup Verification:** The rollup executor validates proofs before including bytecode in HotShot
   batches. Only verified programs may touch Router/slab state.
3. **On-chain Anchoring:** The Solana program stores (bytecode_hash, proof_hash) pairs in the
   registry. Any commit or liquidation CPI includes these hashes, letting auditors confirm that the
   executing code matches the proofed artifact.

---

## 7. Sequencing Advantages

- **Bot Resistance:** HotShot sequencing provides deterministic batch ordering, complementing
  commit-reveal and ARG taxes.
- **Gas-Free UX:** Since everything runs inside the rollup, user transactions consume CurveVM cycles
  instead of Solana CU, enabling high-frequency reserve/commit calls without on-chain fees.
- **Cross-App Interop:** Portfolio accounts share the same rollup ledger used by token launches.
  Users can collateralize CurveScript-generated bonding curves and perp positions under a single
  account tree.

---

## 8. Testing Plan Extensions

All tests from the base design remain mandatory. Additional requirements ensure USP components stay
sound:

- **Compiler Unit Tests:** Verify that CurveScript directives for Router/slab generate expected
  CurveVM bytecode. Include golden tests for reserve/commit loops.
- **Proof Regression:** Every build runs proof generation; CI compares proof hashes to detect drift.
- **VM Execution Tests:** Replay reserve → commit sequences inside CurveVM to ensure instruction
  limits stay below 300k CU and memory stays within 10 MB.
- **Rollup Integration Tests:** Spin up HotShot nodes executing the perp programs alongside bonding
  curve workflows to validate shared portfolio margining.
- **Economic Sims:** Combine token-launch cashflows and perp PnL to ensure CurveVM arithmetic keeps
  equity ≡ cash + Σ pnl even when both apps share collateral.

---

## 9. Deliverables Checklist (Engineering)

- **Router program artifacts:** CurveVM modules covering vault custody, escrow accounting, cap
  issuance, slab registry writes, portfolio margin math, and the liquidation coordinator, each paired
  with proof transcripts that the rollup validator checks before activation.
- **Slab program artifacts:** 10 MB-constrained CurveVM binaries implementing matching, risk
  checks, reservation accounting, commit execution, pending promotion queues, and funding logic for
  every allow-listed LP configuration.
- **On-chain interfaces & client SDKs:** Autogenerated bindings for `reserve`, `commit`, `cancel`,
  `batch_open`, and `liquidation_call` that share the same CurveScript descriptors across Router,
  CurveVM shims, and external client libraries.
- **Oracle adapters:** Shared mark/funding ingestion modules that pipe curated oracle feeds into
  both Router portfolio math and slab funding loops, complete with failover configurations for the
  rollup sequencer.
- **Test harness & CI gating:** End-to-end harness exercising the RM*, M*, L*, and F* suites across
  compiler, CurveVM, rollup, and Anchor layers, wired into CI thresholds for invariant, latency, and
  memory budgets.
- **Benchmarks & soak profiles:** Repeatable load scripts that stress reserve/commit throughput,
  epoch timing, and liquidation flows over 24–72 hour runs while recording VM cycle counts and slab
  memory usage.
- **Formal ACL proofs:** Machine-checked proofs that every debit path respects
  `amount <= min(cap.remaining, escrow.balance)` and that no slab code can reference unrelated
  vaults or escrows.
- **Operational runbooks:** Playbooks for failover, kill-switch invocation, cap expiry monitoring,
  incident response, and proof regeneration that reference sequencer controls and Anchor safety
  levers.

---

## 10. AssetL2 Advantage Statement

By implementing the sharded perp DEX through CurveScript, CurveGPT, the proofed compiler, CurveVM,
and the AssetL2 rollup, we can truthfully claim:
- **Safer Upgrades:** Every slab policy change ships with machine-verifiable proofs.
- **Programmable Liquidity:** LPs express complex routing and anti-toxicity logic via natural-language
  prompts compiled to deterministic VM code.
- **Unified Collateral:** Users post collateral once and reuse it for bonding curves *and* perps with
  provable safety across slabs.
- **Deterministic Ordering:** HotShot sequencing plus commit-reveal eliminates sniping vectors even
  inside high-frequency perp markets.

This alignment keeps the full power of the original design while showcasing AssetL2’s USP as the
only stack that owns the entire code-generation and verification lifecycle.

---

## 11. Alignment with the AssetL2 Mission

- **Own the whole pipeline:** Router and slab programs now ship as CurveVM artifacts generated from
  CurveScript, proving that AssetL2 controls everything from English prompt → script → proofed
  bytecode → rollup execution. The perp architecture therefore extends, rather than dilutes, our
  founding goal of vertically owning code generation and settlement.
- **Safer token launches:** Unified portfolio accounts let bonding-curve capital and perp collateral
  share the same escrow tree. Launches can immediately bootstrap liquidity and hedging markets
  without leaving the verified pipeline, keeping creators safe from bespoke external code.
- **Sequencer leverage:** HotShot batching, already required for token fairness, now amortizes into
  perp epochs. The same deterministic ordering that protects Day-0 buyers also hardens LP routing
  against toxicity, reinforcing the rollup’s differentiator.
- **Composable roadmap:** Because slabs reuse the original state layout, every improvement to
  CurveScript authoring, proof tooling, or CurveVM metering simultaneously benefits token launches
  and perps. The architecture therefore accelerates—not distracts from—our roadmap to own the token
  launch lifecycle end to end.

---

## 12. Competitive Benchmark vs. Purpose-Built Perp Chains

AssetL2 inherits every execution advantage from the reference sharded design while adding USP
capabilities that Hyperliquid-style chains cannot match:

- **AI-native upgrade velocity:** Router/slab policies are described in English, synthesized by
  CurveGPT, and mechanically proven before deployment. Competing chains require manual Rust
  rewrites for each policy shift, introducing human error and multi-week governance cycles.
- **Proof-carrying liquidity programs:** Every upgrade ships with machine-checkable proofs that
  bind cap usage, reservation accounting, and risk arithmetic. Hyperliquid’s architecture relies on
  audits and simulation; AssetL2 can demonstrate safety postures on-chain at deployment time.
- **Unified collateral plane:** Bonding-curve launches and perps share portfolio accounts inside the
  rollup, enabling capital re-use and composable treasury automation. Perp-only chains silo
  collateral per market and cannot natively finance launches out of the same escrow tree.
- **Sequencer composability:** HotShot epochs power both Day-0 launch fairness and perp toxicity
  controls, letting us amortize sequencing innovation across product lines. Purpose-built perp
  chains must choose between trader latency and launch fairness because they lack a shared
  pipeline.
- **Autogenerated client guardrails:** CurveScript descriptors regenerate Router↔slab interfaces and
  SDK bindings. Liquidity providers integrate once and inherit future safeguards automatically,
  whereas other chains leave integrators to hand-maintain bespoke RPC clients.

These differentiators let AssetL2 claim a broader, safer, and faster-moving ecosystem than the
single-purpose perp chains we benchmark against.

---

## 13. Acceptance Criteria (Go/No-Go)

- **Safety:** Demonstrate through unit, fuzz, and formal ACL checks that no execution path can debit
  more than `min(cap.remaining, escrow.balance)` and that slabs remain isolated from collateral they
  do not control.
- **Capital efficiency:** Confirm via regression scenarios that cross-slab hedges keep
  `IM_router <= 1.05 × Σ IM_slab` for canonical long/short offsets, matching the base design’s
  ≤5 % efficiency target.
- **Performance:** Prove with benchmarks that Router `reserve` calls settle in <0.2 ms and
  `commit` calls in <0.5 ms at 50 % book utilization while slab state stays within configured pool
  bounds and below the 10 MB memory cap.
- **Anti-toxicity efficacy:** Validate through historical replays that LP PnL variance improves vs.
  control runs and that sandwich payoff is eliminated inside each batch thanks to commit-reveal,
  kill bands, JIT penalties, and ARG enforcement.
- **Reliability:** Pass chaos suites that cover sequencer restarts, cap expiry churn, and slab
  failover, showing caps expire cleanly and router crash recovery preserves funds with no double
  commits.

