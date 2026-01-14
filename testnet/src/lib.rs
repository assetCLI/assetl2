use assetscript::{emit_manifest, manifest_to_json, parse};
use assetvm::{AssetVM, Instruction, Opcode};
use compiler::{compile_program, parse as parse_curve_program};
use sequencer::{BatchPoster, Consensus, FakeSolanaClient, Mempool, Miner, Tx};
use std::error::Error;

const DEMO_CURVESCRIPT: &str = r#"
ROUTER {
    COLLATERAL asset=USDC vault_cap=50000000
    PORTFOLIO_MARGIN model="cross_alpha" correl_matrix="router::correlations::v1"
    CAP_TTL ms=120000
    RESERVATION_BATCH ms=50
    CAP name="maker" asset=USDC limit=100000000 ttl_ms=60000
}

SLAB "perp:SOL-PERP" {
    MAKER_CLASS DLP allowance=5000000
    MATCHING fifo=true pending_promotion=true
    FEE maker_bps=2 taker_bps=5 rebate_delay_ms=50
    RISK imr_bps=500 mmr_bps=350
    ANTI_TOXICITY kill_band_bps=75 jit_penalty=true arg_tax_bps=10
    BATCH_WINDOW ms=48
    ORACLE_LINK id="pyth:SOLUSD"
}

ORACLE "pyth:SOLUSD" {
    HEARTBEAT ms=500
    KILL_BAND_SYNC router_ref="ROUTER"
}
"#;

pub fn run_demo() -> Result<(i64, String), Box<dyn Error>> {
    let spec = parse(DEMO_CURVESCRIPT)?;
    let manifest = emit_manifest(&spec);
    let manifest_json = manifest_to_json(&manifest)?;
    if manifest_json.len() < 10 {
        return Err("manifest too small".into());
    }

    let asset_prog = vec![
        Instruction {
            opcode: Opcode::Mint,
            amount: 100,
        },
        Instruction {
            opcode: Opcode::Transfer,
            amount: 50,
        },
        Instruction {
            opcode: Opcode::Burn,
            amount: 10,
        },
    ];
    let mut asset_vm = AssetVM::new();
    asset_vm.execute(&asset_prog);

    let curve_script = "BUY 5\nSELL 2\nADD_LIQUIDITY 3\nMIGRATE_TO_AMM 1";
    let curve_cmds = parse_curve_program(curve_script)?;
    let curve_prog = compile_program(&curve_cmds)?;

    let poster = BatchPoster::new(FakeSolanaClient::new());
    let consensus = Consensus::new(vec!["A".into(), "B".into(), "C".into()], poster)?;
    let mut mempool = Mempool::new();
    mempool.add_tx(Tx::new("Alice".into(), 0, curve_prog, "fast".into()));
    let mut miner = Miner::new(mempool, consensus);
    let sig = miner.mine("fast", 1)?;

    Ok((asset_vm.supply, sig))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_manifest_and_router() {
        let spec = parse(DEMO_CURVESCRIPT).unwrap();
        assert_eq!(spec.router.collateral_assets[0].asset, "USDC");
        assert_eq!(spec.slabs[0].oracle.as_deref(), Some("pyth:SOLUSD"));
    }

    #[test]
    fn manifest_serializes_to_json() {
        let spec = parse(DEMO_CURVESCRIPT).unwrap();
        let manifest = emit_manifest(&spec);
        let json = manifest_to_json(&manifest).unwrap();
        assert!(json.contains("reserve"));
    }

    #[test]
    fn asset_vm_still_updates_supply() {
        let asset_prog = vec![
            Instruction {
                opcode: Opcode::Mint,
                amount: 5,
            },
            Instruction {
                opcode: Opcode::Burn,
                amount: 2,
            },
        ];
        let mut vm = AssetVM::new();
        vm.execute(&asset_prog);
        assert_eq!(vm.supply, 3);
    }

    #[test]
    fn parse_curve_script() {
        let cmds = parse_curve_program("BUY 1\nSELL 1").unwrap();
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn compile_curve_program() {
        let cmds = parse_curve_program("BUY 1").unwrap();
        let prog = compile_program(&cmds).unwrap();
        assert_eq!(prog.len(), 1);
    }

    #[test]
    fn mempool_adds_tx() {
        let mut mp = Mempool::new();
        mp.add_tx(Tx::new("A".into(), 0, Vec::new(), "fast".into()));
        let txs = mp.get_txs("fast", 1);
        assert_eq!(txs.len(), 1);
    }

    #[test]
    fn miner_produces_sig() {
        let mut mp = Mempool::new();
        mp.add_tx(Tx::new("A".into(), 0, Vec::new(), "fast".into()));
        let poster = BatchPoster::new(FakeSolanaClient::new());
        let consensus = Consensus::new(vec!["A".into()], poster).unwrap();
        let mut miner = Miner::new(mp, consensus);
        let sig = miner.mine("fast", 1).unwrap();
        assert!(!sig.is_empty());
    }

    #[test]
    fn consensus_height_increments() {
        let poster = BatchPoster::new(FakeSolanaClient::new());
        let mut consensus = Consensus::new(vec!["A".into()], poster).unwrap();
        let block = sequencer::Block {
            program: Vec::new(),
            kind: "fast".into(),
        };
        let sig1 = consensus.propose_and_commit(block).unwrap();
        let block = sequencer::Block {
            program: Vec::new(),
            kind: "fast".into(),
        };
        let sig2 = consensus.propose_and_commit(block).unwrap();
        assert!(!sig1.is_empty() && !sig2.is_empty());
    }

    #[test]
    fn run_demo_returns_supply_and_sig() {
        let (supply, sig) = run_demo().unwrap();
        assert_eq!(supply, 90);
        assert!(!sig.is_empty());
    }

    #[test]
    fn multiple_demo_runs() {
        let (s1, sig1) = run_demo().unwrap();
        let (s2, sig2) = run_demo().unwrap();
        assert_eq!(s1, 90);
        assert_eq!(s2, 90);
        assert!(!sig1.is_empty() && !sig2.is_empty());
    }
}
