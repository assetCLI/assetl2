import json
import hashlib
from dataclasses import dataclass
from typing import List

from ..curvevm import CurveVM
from ..compiler import Instruction
from ..rollup import BatchPoster
from .leader_schedule import round_robin


@dataclass
class Block:
    program: List[Instruction]
    kind: str = "fast"


def _state_root(program: List[Instruction]) -> str:
    vm = CurveVM()
    vm.execute(program)
    state = {
        "balance": vm.balance,
        "liquidity": vm.liquidity,
        "migrated": vm.migrated_to_amm,
        "migrate_value": vm.migrate_value,
    }
    data = json.dumps(state, sort_keys=True).encode("utf-8")
    return hashlib.sha256(data).hexdigest()


class Consensus:
    """A minimal two-phase HotStuff-like loop."""

    def __init__(self, validators: List[str], poster: BatchPoster) -> None:
        if len(validators) < 1:
            raise ValueError("Need at least one validator")
        self.validators = validators
        self.poster = poster
        self._schedule = round_robin(validators)
        self.height = 0

    def propose_and_commit(self, block: Block) -> str:
        leader = next(self._schedule)
        # Each validator executes the block to compute a state root
        roots = {_state_root(block.program) for _ in self.validators}
        if len(roots) != 1:
            raise ValueError("State roots diverged")
        # Post to Solana via BatchPoster
        tx_sig = self.poster.commit(block.program)
        self.height += 1
        return tx_sig
