use borsh::{BorshDeserialize, BorshSerialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum Opcode {
    Mint,
    Transfer,
    Burn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Instruction {
    pub opcode: Opcode,
    pub amount: i64,
}

pub struct AssetVM {
    pub supply: i64,
    pub last_transfer: i64,
}

impl AssetVM {
    pub fn new() -> Self {
        Self { supply: 0, last_transfer: 0 }
    }

    pub fn execute(&mut self, program: &[Instruction]) {
        for ins in program {
            match ins.opcode {
                Opcode::Mint => self.supply += ins.amount,
                Opcode::Transfer => self.last_transfer = ins.amount,
                Opcode::Burn => self.supply -= ins.amount,
            }
        }
    }
}

pub fn program_root(program: &[Instruction]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for ins in program {
        hasher.update(&[ins.opcode as u8]);
        hasher.update(ins.amount.to_le_bytes());
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_execution_and_root() {
        let program = [
            Instruction { opcode: Opcode::Mint, amount: 5 },
            Instruction { opcode: Opcode::Transfer, amount: 2 },
            Instruction { opcode: Opcode::Burn, amount: 1 },
        ];
        let mut vm = AssetVM::new();
        vm.execute(&program);
        assert_eq!(vm.supply, 4);
        assert_eq!(vm.last_transfer, 2);
        let r1 = program_root(&program);
        let r2 = program_root(&program);
        assert_eq!(r1, r2);
    }
}
