from typing import List

from .mempool import Mempool, Tx
from .asset_sequencer_bft import Consensus, Block


class Miner:
    """Consumes mempool transactions and produces blocks."""

    def __init__(self, mempool: Mempool, consensus: Consensus) -> None:
        self.mempool = mempool
        self.consensus = consensus

    def mine(self, kind: str, max_txs: int) -> str:
        txs = self.mempool.get_txs(kind, max_txs)
        program: List = []
        for tx in txs:
            program.extend(tx.program)
        block = Block(program=program, kind=kind)
        return self.consensus.propose_and_commit(block)
