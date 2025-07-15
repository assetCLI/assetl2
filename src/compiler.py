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
    for cmd in commands:
        if cmd.opcode.upper() != "BUY":
            raise ValueError(f"Unknown command: {cmd.opcode}")
        instructions.append(Instruction(opcode="BUY", operand=cmd.operand))
    return instructions
