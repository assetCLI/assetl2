from typing import List
from .compiler import Instruction


class CurveVM:
    """A minimal VM supporting only the BUY opcode."""

    def __init__(self) -> None:
        self.balance = 0

    def execute(self, program: List[Instruction]) -> None:
        for ins in program:
            if ins.opcode.upper() == "BUY":
                self.balance += ins.operand
            else:
                raise ValueError(f"Unknown opcode: {ins.opcode}")
