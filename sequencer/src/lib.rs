use compiler::Instruction;
use curvevm::{CurveVM, Opcode};
use hotshot::HotShotConsensus;
use std::time::{Duration, SystemTime};

pub struct Mempool {
    fast_pool: Vec<Tx>,
    big_pool: Vec<Tx>,
}

pub struct Tx {
    pub sender: String,
    pub nonce: u64,
    pub program: Vec<Instruction>,
    pub kind: String,
    pub timestamp: SystemTime,
}

impl Tx {
    pub fn new(sender: String, nonce: u64, program: Vec<Instruction>, kind: String) -> Self {
        Self { sender, nonce, program, kind, timestamp: SystemTime::now() }
    }
}

impl Mempool {
    pub fn new() -> Self {
        Self { fast_pool: Vec::new(), big_pool: Vec::new() }
    }

    fn pool(&mut self, kind: &str) -> &mut Vec<Tx> {
        match kind {
            "fast" => &mut self.fast_pool,
            "big" => &mut self.big_pool,
            _ => panic!("Unknown block type"),
        }
    }

    fn prune(&mut self) {
        let cutoff = SystemTime::now() - Duration::from_secs(86_400);
        self.fast_pool.retain(|t| t.timestamp >= cutoff);
        self.big_pool.retain(|t| t.timestamp >= cutoff);
    }

    pub fn add_tx(&mut self, tx: Tx) {
        self.prune();
        let pool = self.pool(&tx.kind);
        let nonces: Vec<_> = pool.iter().filter(|t| t.sender == tx.sender).map(|t| t.nonce).collect();
        if nonces.len() >= 8 && !nonces.contains(&tx.nonce) {
            panic!("Nonce window exceeded");
        }
        pool.push(tx);
    }

    pub fn get_txs(&mut self, kind: &str, limit: usize) -> Vec<Tx> {
        self.prune();
        let pool = self.pool(kind);
        let txs = pool.drain(0..limit.min(pool.len())).collect();
        txs
    }
}

pub struct Block {
    pub program: Vec<Instruction>,
    pub kind: String,
}

pub struct FakePoster;
impl FakePoster { pub fn commit(&self, _program: &[Instruction]) -> String { "sig".into() } }

pub struct Consensus {
    engine: HotShotConsensus,
    poster: FakePoster,
}

impl Consensus {
    pub fn new(poster: FakePoster) -> Self {
        Self { engine: HotShotConsensus::new(), poster }
    }

    pub fn propose_and_commit(&mut self, block: Block) -> String {
        self.engine.commit_block(&block.program);
        self.poster.commit(&block.program)
    }
}


pub struct Miner {
    mp: Mempool,
    consensus: Consensus,
}

impl Miner {
    pub fn new(mp: Mempool, consensus: Consensus) -> Self { Self { mp, consensus } }

    pub fn mine(&mut self, kind: &str, max_txs: usize) -> String {
        let txs = self.mp.get_txs(kind, max_txs);
        let mut program = Vec::new();
        for tx in txs { program.extend(tx.program); }
        let block = Block { program, kind: kind.into() };
        self.consensus.propose_and_commit(block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mempool_nonce_and_prune() {
        let mut mp = Mempool::new();
        let program = vec![Instruction { opcode: Opcode::Buy, operand: 1 }];
        let mut old = Tx::new("A".into(), 0, program.clone(), "fast".into());
        old.timestamp = SystemTime::now() - Duration::from_secs(90_000);
        mp.add_tx(old);
        for n in 1..8 { mp.add_tx(Tx::new("A".into(), n, program.clone(), "fast".into())); }
        mp.add_tx(Tx::new("A".into(), 8, program.clone(), "fast".into()));
        assert_eq!(mp.fast_pool.len(), 8);
    }

    #[test]
    fn miner_mines_block() {
        let mut mp = Mempool::new();
        let program = vec![Instruction { opcode: Opcode::Buy, operand: 1 }];
        mp.add_tx(Tx::new("A".into(), 0, program.clone(), "fast".into()));
        let poster = FakePoster;
        let mut consensus = Consensus::new(poster);
        let mut miner = Miner::new(mp, consensus);
        let sig = miner.mine("fast", 1);
        assert_eq!(sig, "sig");
    }
}
