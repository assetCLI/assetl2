from dataclasses import dataclass
from typing import List
from .curvescript import Command

@dataclass
class Instruction:
    opcode: str
    operand: int


def compile_program(commands: List[Command]) -> List[Instruction]:
    """Compile a list of Commands into CurveVM instructions."""
    instructions: List[Instruction] = []
    allowed = {"BUY", "SELL", "ADD_LIQUIDITY", "MIGRATE_TO_AMM"}
    for cmd in commands:
        op = cmd.opcode.upper()
        if op not in allowed:
            raise ValueError(f"Unknown command: {cmd.opcode}")
        instructions.append(Instruction(opcode=op, operand=cmd.operand))
    return instructions
