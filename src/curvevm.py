from typing import List
from .compiler import Instruction


class CurveVM:
    """A minimal VM supporting four opcodes."""

    def __init__(self) -> None:
        self.balance = 0
        self.liquidity = 0
        self.migrated_to_amm = False
        self.migrate_value = 0

    def execute(self, program: List[Instruction]) -> None:
        for ins in program:
            op = ins.opcode.upper()
            if op == "BUY":
                self.balance += ins.operand
            elif op == "SELL":
                self.balance -= ins.operand
            elif op == "ADD_LIQUIDITY":
                self.liquidity += ins.operand
            elif op == "MIGRATE_TO_AMM":
                self.migrated_to_amm = True
                self.migrate_value = ins.operand
            else:
                raise ValueError(f"Unknown opcode: {ins.opcode}")
