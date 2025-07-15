from dataclasses import dataclass
from typing import List

@dataclass
class Command:
    opcode: str
    operand: int


def parse(script: str) -> List[Command]:
    """Parse a tiny CurveScript supporting four opcodes."""
    commands: List[Command] = []
    allowed = {"BUY", "SELL", "ADD_LIQUIDITY", "MIGRATE_TO_AMM"}
    for line in script.strip().splitlines():
        line = line.strip()
        if not line:
            continue
        parts = line.split()
        if len(parts) != 2 or parts[0].upper() not in allowed:
            raise ValueError(f"Invalid statement: {line}")
        try:
            amount = int(parts[1])
        except ValueError as e:
            raise ValueError(f"Invalid amount in: {line}") from e
        commands.append(Command(opcode=parts[0].upper(), operand=amount))
    return commands
