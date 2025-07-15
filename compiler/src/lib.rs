use curvevm::{Opcode, Instruction as VmInstruction};
pub type Instruction = VmInstruction;

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Command {
    pub opcode: String,
    pub operand: i64,
}



pub fn parse(script: &str) -> Result<Vec<Command>, String> {
    let mut commands = Vec::new();
    let allowed = ["BUY", "SELL", "ADD_LIQUIDITY", "MIGRATE_TO_AMM"];
    for line in script.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 2 || !allowed.contains(&parts[0].to_ascii_uppercase().as_str()) {
            return Err(format!("Invalid statement: {}", line));
        }
        let amount: i64 = parts[1].parse().map_err(|_| format!("Invalid amount in: {}", line))?;
        commands.push(Command { opcode: parts[0].to_ascii_uppercase(), operand: amount });
    }
    Ok(commands)
}

pub fn compile_program(commands: &[Command]) -> Result<Vec<Instruction>, String> {
    commands
        .iter()
        .map(|cmd| {
            let opcode = match cmd.opcode.as_str() {
                "BUY" => Opcode::Buy,
                "SELL" => Opcode::Sell,
                "ADD_LIQUIDITY" => Opcode::AddLiquidity,
                "MIGRATE_TO_AMM" => Opcode::MigrateToAmm,
                other => return Err(format!("Unknown command: {}", other)),
            };
            Ok(Instruction { opcode, operand: cmd.operand })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_compile() {
        let script = "BUY 5\nSELL 2\nADD_LIQUIDITY 3\nMIGRATE_TO_AMM 1";
        let cmds = parse(script).unwrap();
        let program = compile_program(&cmds).unwrap();
        assert_eq!(program.len(), 4);
        assert_eq!(program[0].opcode, Opcode::Buy);
        assert_eq!(program[1].opcode, Opcode::Sell);
        assert_eq!(program[2].opcode, Opcode::AddLiquidity);
        assert_eq!(program[3].opcode, Opcode::MigrateToAmm);
    }
}
