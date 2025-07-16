use assetvm::{Instruction, Opcode};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Command {
    pub opcode: String,
    pub amount: i64,
}

pub fn parse(script: &str) -> Result<Vec<Command>, String> {
    let mut cmds = Vec::new();
    let allowed = ["MINT", "TRANSFER", "BURN"];
    for line in script.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 2 || !allowed.contains(&parts[0].to_ascii_uppercase().as_str()) {
            return Err(format!("Invalid statement: {}", line));
        }
        let amount: i64 = parts[1]
            .parse()
            .map_err(|_| format!("Invalid amount in: {}", line))?;
        cmds.push(Command { opcode: parts[0].to_ascii_uppercase(), amount });
    }
    Ok(cmds)
}

pub fn compile(commands: &[Command]) -> Result<Vec<Instruction>, String> {
    commands
        .iter()
        .map(|cmd| {
            let op = match cmd.opcode.as_str() {
                "MINT" => Opcode::Mint,
                "TRANSFER" => Opcode::Transfer,
                "BURN" => Opcode::Burn,
                other => return Err(format!("Unknown command: {}", other)),
            };
            Ok(Instruction { opcode: op, amount: cmd.amount })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assetvm::{AssetVM, program_root};

    #[test]
    fn parse_compile_execute_pipeline() {
        let script = "MINT 5\nTRANSFER 3\nBURN 1";
        let cmds = parse(script).unwrap();
        let program = compile(&cmds).unwrap();
        let mut vm = AssetVM::new();
        vm.execute(&program);
        assert_eq!(vm.supply, 4);
        assert_eq!(vm.last_transfer, 3);
        let _root = program_root(&program);
    }
}
