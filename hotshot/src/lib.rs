use compiler::Instruction;

#[derive(Default)]
pub struct HotShotConsensus {
    pub height: u64,
}

impl HotShotConsensus {
    pub fn new() -> Self {
        Self { height: 0 }
    }

    pub fn commit_block(&mut self, _block: &[Instruction]) -> u64 {
        self.height += 1;
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvevm::Opcode;

    #[test]
    fn commits_blocks() {
        let mut hs = HotShotConsensus::new();
        let block = vec![Instruction { opcode: Opcode::Buy, operand: 1 }];
        let h1 = hs.commit_block(&block);
        let h2 = hs.commit_block(&block);
        assert_eq!(h1, 1);
        assert_eq!(h2, 2);
    }
}
