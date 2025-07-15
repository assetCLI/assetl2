from dataclasses import dataclass, field
from typing import Dict, List
import time

from .compiler import Instruction


@dataclass
class Tx:
    sender: str
    nonce: int
    program: List[Instruction]
    kind: str = "fast"
    timestamp: float = field(default_factory=time.time)


class Mempool:
    """Dual mempool for fast and big blocks."""

    def __init__(self) -> None:
        self.fast_pool: List[Tx] = []
        self.big_pool: List[Tx] = []

    def _pool(self, kind: str) -> List[Tx]:
        if kind == "fast":
            return self.fast_pool
        elif kind == "big":
            return self.big_pool
        else:
            raise ValueError("Unknown block type")

    def _prune(self) -> None:
        cutoff = time.time() - 86400  # 24h
        self.fast_pool = [tx for tx in self.fast_pool if tx.timestamp >= cutoff]
        self.big_pool = [tx for tx in self.big_pool if tx.timestamp >= cutoff]

    def add_tx(self, tx: Tx) -> None:
        self._prune()
        pool = self._pool(tx.kind)
        # Enforce at most 8 pending nonces per sender
        nonces = [t.nonce for t in pool if t.sender == tx.sender]
        if len(set(nonces)) >= 8 and tx.nonce not in nonces:
            raise ValueError("Nonce window exceeded")
        pool.append(tx)

    def get_txs(self, kind: str, limit: int) -> List[Tx]:
        self._prune()
        pool = self._pool(kind)
        txs = pool[:limit]
        del pool[:limit]
        return txs
