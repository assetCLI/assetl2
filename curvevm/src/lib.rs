use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum Opcode {
    Buy,
    Sell,
    AddLiquidity,
    MigrateToAmm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operand: i64,
}

pub struct CurveVM {
    pub balance: i64,
    pub liquidity: i64,
    pub migrated_to_amm: bool,
    pub migrate_value: i64,
}

impl CurveVM {
    pub fn new() -> Self {
        Self { balance: 0, liquidity: 0, migrated_to_amm: false, migrate_value: 0 }
    }

    pub fn execute(&mut self, program: &[Instruction]) {
        for ins in program {
            match ins.opcode {
                Opcode::Buy => self.balance += ins.operand,
                Opcode::Sell => self.balance -= ins.operand,
                Opcode::AddLiquidity => self.liquidity += ins.operand,
                Opcode::MigrateToAmm => {
                    self.migrated_to_amm = true;
                    self.migrate_value = ins.operand;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_program() {
        let program = [
            Instruction { opcode: Opcode::Buy, operand: 5 },
            Instruction { opcode: Opcode::Sell, operand: 2 },
            Instruction { opcode: Opcode::AddLiquidity, operand: 3 },
            Instruction { opcode: Opcode::MigrateToAmm, operand: 1 },
        ];
        let mut vm = CurveVM::new();
        vm.execute(&program);
        assert_eq!(vm.balance, 3);
        assert_eq!(vm.liquidity, 3);
        assert!(vm.migrated_to_amm);
        assert_eq!(vm.migrate_value, 1);
    }
}
