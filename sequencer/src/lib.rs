use base64::{Engine as _, engine::general_purpose};
use compiler::Instruction;
use curvevm::{CurveVM, Opcode};
use hotshot::HotShotConsensus;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
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

pub struct RoundRobin<T> {
    vals: Vec<T>,
    idx: usize,
}

impl<T: Clone> RoundRobin<T> {
    pub fn new(vals: Vec<T>) -> Result<Self, String> {
        if vals.is_empty() {
            return Err("No validators provided".into());
        }
        Ok(Self { vals, idx: 0 })
    }
}

impl<T: Clone> Iterator for RoundRobin<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let v = self.vals[self.idx % self.vals.len()].clone();
        self.idx += 1;
        Some(v)
    }
}

fn state_root(program: &[Instruction]) -> String {
    let mut vm = CurveVM::new();
    vm.execute(program);
    let state = json!({
        "balance": vm.balance,
        "liquidity": vm.liquidity,
        "migrated": vm.migrated_to_amm,
        "migrate_value": vm.migrate_value,
    });
    let encoded = serde_json::to_vec(&state).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&encoded);
    format!("{:x}", hasher.finalize())
}

pub struct FakeSolanaClient {
    pub sent: Vec<String>,
}

impl FakeSolanaClient {
    pub fn new() -> Self {
        Self { sent: Vec::new() }
    }

    pub fn send_transaction(&mut self, data: &[u8]) -> String {
        let b64 = general_purpose::STANDARD.encode(data);
        self.sent.push(b64.clone());
        b64
    }
}

pub struct BatchPoster {
    pub client: FakeSolanaClient,
}

impl BatchPoster {
    pub fn new(client: FakeSolanaClient) -> Self {
        Self { client }
    }

    pub fn commit(&mut self, program: &[Instruction]) -> String {
        #[derive(Serialize)]
        struct Instr<'a> {
            op: &'a str,
            arg: i64,
        }
        use std::collections::BTreeMap;
        let list: Vec<BTreeMap<&str, serde_json::Value>> = program
            .iter()
            .map(|ins| {
                let mut map = BTreeMap::new();
                let op_str = match ins.opcode {
                    Opcode::Buy => "BUY",
                    Opcode::Sell => "SELL",
                    Opcode::AddLiquidity => "ADD_LIQUIDITY",
                    Opcode::MigrateToAmm => "MIGRATE_TO_AMM",
                };
                map.insert("op", serde_json::Value::String(op_str.into()));
                map.insert("arg", serde_json::Value::from(ins.operand));
                map
            })
            .collect();
        let encoded = serde_json::to_string(&list).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(encoded.as_bytes());
        let root = format!("{:x}", hasher.finalize());
        let payload = json!({ "root": root, "program": list });
        let data = serde_json::to_vec(&payload).unwrap();
        self.client.send_transaction(&data)
    }
}

pub struct FakePoster; // Backwards compatibility
impl FakePoster {
    pub fn commit(&self, _program: &[Instruction]) -> String {
        "sig".into()
    }
}

pub struct Consensus {
    engine: HotShotConsensus,
    pub poster: BatchPoster,
    validators: Vec<String>,
    schedule: RoundRobin<String>,
    state_root_hook: Option<Box<dyn FnMut(&[Instruction]) -> String>>,
}

impl Consensus {
    pub fn new(validators: Vec<String>, poster: BatchPoster) -> Result<Self, String> {
        let schedule = RoundRobin::new(validators.clone())?;
        Ok(Self {
            engine: HotShotConsensus::new(),
            poster,
            validators,
            schedule,
            state_root_hook: None,
        })
    }

    fn compute_root(&mut self, program: &[Instruction]) -> String {
        if let Some(hook) = self.state_root_hook.as_mut() {
            hook(program)
        } else {
            state_root(program)
        }
    }

    pub fn set_state_root_hook<F>(&mut self, f: F)
    where
        F: FnMut(&[Instruction]) -> String + 'static,
    {
        self.state_root_hook = Some(Box::new(f));
    }

    pub fn propose_and_commit(&mut self, block: Block) -> Result<String, String> {
        let _leader = self.schedule.next().unwrap();
        let mut roots = std::collections::HashSet::new();
        for _ in 0..self.validators.len() {
            roots.insert(self.compute_root(&block.program));
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
        let poster = BatchPoster::new(FakeSolanaClient::new());
        let mut consensus = Consensus::new(vec!["A".into()], poster).unwrap();
        let mut miner = Miner::new(mp, consensus);
        let sig = miner.mine("fast", 1).unwrap();
        assert_eq!(sig, *miner.consensus.poster.client.sent.last().unwrap());
    }

    #[test]
    fn round_robin_schedule() {
        let mut rr =
            RoundRobin::new(vec!["A".to_string(), "B".to_string(), "C".to_string()]).unwrap();
        let vals: Vec<_> = (0..5).map(|_| rr.next().unwrap()).collect();
        assert_eq!(vals, ["A", "B", "C", "A", "B"]);
    }

    #[test]
    fn consensus_commit() {
        let program = vec![Instruction {
            opcode: Opcode::Buy,
            operand: 1,
        }];
        let poster = BatchPoster::new(FakeSolanaClient::new());
        let mut consensus =
            Consensus::new(vec!["A".into(), "B".into(), "C".into()], poster).unwrap();
        let block = Block {
            program: program.clone(),
            kind: "fast".into(),
        };
        let tx = consensus.propose_and_commit(block).unwrap();
        let sent = consensus.poster.client.sent.last().unwrap();
        assert_eq!(tx, *sent);
        let data = general_purpose::STANDARD.decode(tx).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&data).unwrap();
        let instr_list = v.get("program").unwrap();
        let encoded = serde_json::to_string(instr_list).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(encoded.as_bytes());
        let expected_root = format!("{:x}", hasher.finalize());
        assert_eq!(v.get("root").unwrap().as_str().unwrap(), expected_root);
    }

    #[test]
    fn consensus_no_validators() {
        let poster = BatchPoster::new(FakeSolanaClient::new());
        let res = Consensus::new(vec![], poster);
        assert!(res.is_err());
    }

    #[test]
    fn state_root_divergence() {
        let program = vec![Instruction {
            opcode: Opcode::Buy,
            operand: 1,
        }];
        let poster = BatchPoster::new(FakeSolanaClient::new());
        let mut consensus = Consensus::new(vec!["A".into(), "B".into()], poster).unwrap();
        let counter = std::cell::Cell::new(0);
        consensus.set_state_root_hook(move |_| {
            let c = counter.get();
            counter.set(c + 1);
            if c == 0 { "a".into() } else { "b".into() }
        });
        let block = Block {
            program,
            kind: "fast".into(),
        };
        assert!(consensus.propose_and_commit(block).is_err());
    }
}
