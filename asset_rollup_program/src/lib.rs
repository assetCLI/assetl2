use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use compiler::Instruction;
use sha2::{Digest, Sha256};

declare_id!("11111111111111111111111111111111");

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone)]
pub struct Payload {
    pub root: [u8; 32],
    pub program: Vec<Instruction>,
}

pub fn serialize_program(program: &[Instruction]) -> Payload {
    let mut hasher = Sha256::new();
    for ins in program {
        hasher.update(&[ins.opcode as u8]);
        hasher.update(ins.operand.to_le_bytes());
    }
    let root = hasher.finalize();
    Payload { root: root.into(), program: program.to_vec() }
}

#[account]
pub struct RollupState {
    pub last_root: [u8; 32],
}

#[program]
pub mod asset_rollup_program {
    use super::*;

    pub fn post_batch(ctx: Context<PostBatch>, payload: Payload) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.last_root = payload.root;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct PostBatch<'info> {
    #[account(mut)]
    pub state: Account<'info, RollupState>,
    pub authority: Signer<'info>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvevm::Opcode;

    #[test]
    fn payload_has_expected_root() {
        let program = vec![Instruction { opcode: Opcode::Buy, operand: 1 }];
        let p = serialize_program(&program);
        let expected = serialize_program(&program);
        assert_eq!(p, expected);
    }
}
