Hi, I’m the founder of **AssetCLI**, the team that just won Solana’s AI-Agent MCP hackathon for re-imagining token launches.

On-chain capital markets are exploding.  Hyperliquid’s purpose-built exchange chain cleared **US \$248 billion in perpetual-futures volume last month, a 51 % jump in thirty days** ([tokeninsight.com][1], [x.com][2]).  Meanwhile Pump․fun’s memecoin factory is racing toward a **billion-dollar annual run-rate** as Solana’s top-grossing app ([wired.com][3], [news.bitcoin.com][4]).  Yet the first block of most launches is still hijacked by Telegram snipers, who vacuum up 80 % of supply and dump on real fans ([blockworks.co][5], [x.com][6]).

Why does this happen?  Creators must hand over control to generic launchpads, and changing a curve still means writing low-level Solana programs—something large-language models compile correctly less than a third of the time on public benchmarks ([arxiv.org][7]).

**AssetCLI fixes that today.**  Our live agent turns a plain-English prompt—“raise a million dollars on a linear curve” —into production-ready Solana code: mint, curve maths, automatic Raydium pool.  

**Tomorrow we go beyond code into vertical control just like  Hyperliquid.**  Hyperliquid hard-wires an order-book into its own chain to give traders deterministic sequencing and zero gas. AssetCLI does the same for token launchpads, but we hard-wire the code-gen pipeline itself

1. CurveScript. A 30-keyword DSL with no syscalls or signer arrays, shrinking the LLM search space by ~100× and scoring > 95 % first-compile success in our pilots

2. CurveGPT. The agent that writes only CurveScript from plain English prompts like “Raise $1 million on a linear curve that auto-migrates at 300 SOL reserve.

3. Compiler + Proof Engine. Converts that script into WASM and proof of overflow-free maths and non-negative reserves before we let it anywhere near main-net .

4. CurveVM. A micro-VM inside our roll-up that runs only four op-codes—buy, sell, add_liquidity, migrate_to_amm—so every call stays below 300 k compute units versus Solana’s 1.4 M ceiling and executes in parallel  with zero account collisions 

* **AssetL2 roll-up.**  Anchored to Solana but sequenced by AssetSequencer-BFT - a Hyperliquid style consensus that ultimately settles to Solana every second and gives you bot-proof ordering of transactions.

```python
from src.curvescript import parse
from src.compiler import compile_program
from src.curvevm import CurveVM
from src.rollup import BatchPoster, FakeSolanaClient

script = """BUY 5\nSELL 2\nADD_LIQUIDITY 3\nMIGRATE_TO_AMM 1"""
program = compile_program(parse(script))
vm = CurveVM()
vm.execute(program)
client = FakeSolanaClient()
BatchPoster(client).commit(program)
```


Hyperliquid’s edge is owning the trading state machine; **our edge is owning the *code-generation* state machine.**  Creators get deterministic first-block fairness, gas-free UX, and a cryptographic proof that the AI-written curve can’t rug pull - these are advantages that generic pads and generic code-gen simply can’t match.

If you believe the next Nasdaq will be *described* in chat, not coded in Rust, join us.  **AssetCLI lets you speak your raise today and will guarantee it’s safe tomorrow.**  Thank you.

[1]: https://tokeninsight.com/en/news/hyperliquid-hits-record-248-billion-perp-volume-in-may-capturing-over-10-of-binance-flow?utm_source=chatgpt.com "Hyperliquid hits record $248 billion perp volume in May, capturing ..."
[2]: https://x.com/cryptonewsz_/status/1931312408456257895?utm_source=chatgpt.com "CryptoNewsZ - X"
[3]: https://www.wired.com/story/madcap-rise-of-memecoin-factory-pumpfun?utm_source=chatgpt.com "The Madcap Rise of Memecoin Factory Pump.Fun"
[4]: https://news.bitcoin.com/pump-fun-leads-revenue-surge-as-solana-has-best-quarter-in-12-months/?utm_source=chatgpt.com "Pump.fun Leads Revenue Surge as Solana Has Best Quarter in 12 ..."
[5]: https://blockworks.co/news/pumpfun-raise-acquisition-companies?utm_source=chatgpt.com "4 companies pump.fun could look at acquiring after its $1B raise"
[6]: https://x.com/soulscannerbot?utm_source=chatgpt.com "Soul Sniper (@soulscannerbot) / X"
[7]: https://arxiv.org/html/2506.03006v2?utm_source=chatgpt.com "A Preference-Driven Methodology for High-Quality Solidity Code ..."
[8]: https://www.theblock.co/post/346512/solana-marks-5-year-anniversary-as-network-activity-dips-firedancer-launch-inches-closer?utm_source=chatgpt.com "Solana marks 5 year anniversary as network activity dips ... - The Block"
[9]: https://solana.stackexchange.com/questions/18346/example-contract-code-for-a-bonding-curve?utm_source=chatgpt.com "Example contract code for a bonding curve? - Solana Stack Exchange"
[10]: https://arxiv.org/html/2403.18300v1?utm_source=chatgpt.com "HotStuff-2 vs. HotStuff: The Difference and Advantage - arXiv"
[11]: https://dune.com/jhackworth/pumpfun?utm_source=chatgpt.com "Pump.Fun"
