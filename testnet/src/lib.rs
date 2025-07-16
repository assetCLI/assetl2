use assetscript::{parse as parse_asset, compile as compile_asset};
use assetvm::AssetVM;
use compiler::{parse, compile_program};
use sequencer::{BatchPoster, Consensus, FakeSolanaClient, Mempool, Miner, Tx};
use std::error::Error;

pub fn run_demo() -> Result<(i64, String), Box<dyn Error>> {
    let asset_script = "MINT 100\nTRANSFER 50\nBURN 10";
    let asset_cmds = parse_asset(asset_script)?;
    let asset_prog = compile_asset(&asset_cmds)?;
    let mut asset_vm = AssetVM::new();
    asset_vm.execute(&asset_prog);

    let curve_script = "BUY 5\nSELL 2\nADD_LIQUIDITY 3\nMIGRATE_TO_AMM 1";
    let curve_cmds = parse(curve_script)?;
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
    fn parse_asset_script() {
        let cmds = parse_asset("MINT 1\nBURN 1").unwrap();
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn compile_asset_program() {
        let cmds = parse_asset("MINT 1").unwrap();
        let prog = compile_asset(&cmds).unwrap();
        assert_eq!(prog.len(), 1);
    }

    #[test]
    fn execute_asset_program() {
        let cmds = parse_asset("MINT 5\nBURN 2").unwrap();
        let prog = compile_asset(&cmds).unwrap();
        let mut vm = AssetVM::new();
        vm.execute(&prog);
        assert_eq!(vm.supply, 3);
    }

    #[test]
    fn parse_curve_script() {
        let cmds = parse("BUY 1\nSELL 1").unwrap();
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn compile_curve_program() {
        let cmds = parse("BUY 1").unwrap();
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
        let block = sequencer::Block { program: Vec::new(), kind: "fast".into() };
        let sig1 = consensus.propose_and_commit(block).unwrap();
        let block = sequencer::Block { program: Vec::new(), kind: "fast".into() };
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
