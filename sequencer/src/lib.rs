use compiler::Instruction;
use curvevm::{CurveVM, Opcode};
use hotshot::HotShotConsensus;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::time::{Duration, SystemTime};

#[derive(Clone)]
pub struct RoundRobin {
    validators: Vec<String>,
    idx: usize,
}

impl RoundRobin {
    pub fn new(validators: Vec<String>) -> Self {
        Self { validators, idx: 0 }
    }
}

impl Iterator for RoundRobin {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        if self.validators.is_empty() {
            return None;
        }
        let v = self.validators[self.idx % self.validators.len()].clone();
        self.idx += 1;
        Some(v)
    }
}

#[derive(Serialize)]
struct VmState {
    balance: i64,
    liquidity: i64,
    migrated: bool,
    migrate_value: i64,
}

fn state_root(program: &[Instruction]) -> [u8; 32] {
    let mut vm = CurveVM::new();
    vm.execute(program);
    let state = VmState {
        balance: vm.balance,
        liquidity: vm.liquidity,
        migrated: vm.migrated_to_amm,
        migrate_value: vm.migrate_value,
    };
    let data = serde_json::to_vec(&state).unwrap();
    let hash = Sha256::digest(&data);
    hash.into()
}

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
        Self {
            sender,
            nonce,
            program,
            kind,
            timestamp: SystemTime::now(),
        }
    }
}

impl Mempool {
    pub fn new() -> Self {
        Self {
            fast_pool: Vec::new(),
            big_pool: Vec::new(),
        }
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
        let nonces: Vec<_> = pool
            .iter()
            .filter(|t| t.sender == tx.sender)
            .map(|t| t.nonce)
            .collect();
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
impl FakePoster {
    pub fn commit(&self, _program: &[Instruction]) -> String {
        "sig".into()
    }
}

pub struct Consensus {
    validators: Vec<String>,
    schedule: RoundRobin,
    engine: HotShotConsensus,
    poster: FakePoster,
    state_root_fn: Box<dyn Fn(&[Instruction]) -> [u8; 32]>,
}

impl Consensus {
    pub fn new(
        validators: Vec<String>,
        poster: FakePoster,
        state_root_fn: Box<dyn Fn(&[Instruction]) -> [u8; 32]>,
    ) -> Result<Self, String> {
        if validators.is_empty() {
            return Err("Need at least one validator".into());
        }
        let schedule = RoundRobin::new(validators.clone());
        Ok(Self {
            validators,
            schedule,
            engine: HotShotConsensus::new(),
            poster,
            state_root_fn,
        })
    }

    pub fn propose_and_commit(&mut self, block: Block) -> Result<String, String> {
        let _leader = self.schedule.next().unwrap();
        let mut roots = HashSet::new();
        for _ in &self.validators {
            roots.insert((self.state_root_fn)(&block.program));
        }
        if roots.len() != 1 {
            return Err("State roots diverged".into());
        }
        self.engine.commit_block(&block.program);
        Ok(self.poster.commit(&block.program))
    }
}

pub struct Miner {
    mp: Mempool,
    consensus: Consensus,
}

impl Miner {
    pub fn new(mp: Mempool, consensus: Consensus) -> Self {
        Self { mp, consensus }
    }

    pub fn mine(&mut self, kind: &str, max_txs: usize) -> Result<String, String> {
        let txs = self.mp.get_txs(kind, max_txs);
        let mut program = Vec::new();
        for tx in txs {
            program.extend(tx.program);
        }
        let block = Block {
            program,
            kind: kind.into(),
        };
        self.consensus.propose_and_commit(block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_robin_schedule() {
        let mut sched = RoundRobin::new(vec!["A".into(), "B".into(), "C".into()]);
        assert_eq!(sched.next().as_deref(), Some("A"));
        assert_eq!(sched.next().as_deref(), Some("B"));
        assert_eq!(sched.next().as_deref(), Some("C"));
        assert_eq!(sched.next().as_deref(), Some("A"));
        assert_eq!(sched.next().as_deref(), Some("B"));
    }

    #[test]
    fn consensus_commit() {
        let program = vec![Instruction {
            opcode: Opcode::Buy,
            operand: 1,
        }];
        let poster = FakePoster;
        let state_fn = Box::new(state_root as fn(&[Instruction]) -> [u8; 32]);
        let mut consensus = Consensus::new(vec!["A".into()], poster, state_fn).unwrap();
        let block = Block {
            program: program.clone(),
            kind: "fast".into(),
        };
        let sig = consensus.propose_and_commit(block).unwrap();
        assert_eq!(sig, "sig");
        assert_eq!(consensus.engine.height, 1);
    }

    #[test]
    fn state_root_divergence() {
        use std::cell::RefCell;
        let program = vec![Instruction {
            opcode: Opcode::Buy,
            operand: 1,
        }];
        let poster = FakePoster;
        let counter = RefCell::new(0);
        let root_fn = Box::new(move |_: &[Instruction]| {
            let mut c = counter.borrow_mut();
            *c += 1;
            if *c == 1 { [1u8; 32] } else { [2u8; 32] }
        });
        let mut consensus = Consensus::new(vec!["A".into(), "B".into()], poster, root_fn).unwrap();
        let block = Block {
            program,
            kind: "fast".into(),
        };
        let err = consensus.propose_and_commit(block).unwrap_err();
        assert_eq!(err, "State roots diverged");
    }

    #[test]
    fn mempool_nonce_and_prune() {
        let mut mp = Mempool::new();
        let program = vec![Instruction {
            opcode: Opcode::Buy,
            operand: 1,
        }];
        let mut old = Tx::new("A".into(), 0, program.clone(), "fast".into());
        old.timestamp = SystemTime::now() - Duration::from_secs(90_000);
        mp.add_tx(old);
        for n in 1..8 {
            mp.add_tx(Tx::new("A".into(), n, program.clone(), "fast".into()));
        }
        mp.add_tx(Tx::new("A".into(), 8, program.clone(), "fast".into()));
        assert_eq!(mp.fast_pool.len(), 8);
    }

    #[test]
    fn miner_mines_block() {
        let mut mp = Mempool::new();
        let program = vec![Instruction {
            opcode: Opcode::Buy,
            operand: 1,
        }];
        mp.add_tx(Tx::new("A".into(), 0, program.clone(), "fast".into()));
        let poster = FakePoster;
        let state_fn = Box::new(state_root as fn(&[Instruction]) -> [u8; 32]);
        let mut consensus = Consensus::new(vec!["A".into()], poster, state_fn).unwrap();
        let mut miner = Miner::new(mp, consensus);
        let sig = miner.mine("fast", 1).unwrap();
        assert_eq!(sig, "sig");
    }
}
