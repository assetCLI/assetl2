import os
import sys
import time
import pytest

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from src.mempool import Mempool, Tx
from src.compiler import Instruction


def test_mempool_nonce_and_prune():
    mp = Mempool()
    program = [Instruction("BUY", 1)]
    old = Tx(sender="A", nonce=0, program=program)
    old.timestamp -= 90000
    mp.add_tx(old)
    for n in range(1, 8):
        mp.add_tx(Tx(sender="A", nonce=n, program=program))
    mp.add_tx(Tx(sender="A", nonce=8, program=program))
    assert len(mp.fast_pool) == 8  # old pruned
    assert all(t.timestamp >= time.time() - 86400 for t in mp.fast_pool)


def test_mempool_dual_queues():
    mp = Mempool()
    program = [Instruction("BUY", 1)]
    mp.add_tx(Tx(sender="A", nonce=0, program=program, kind="big"))
    assert mp.get_txs("big", 1)[0].kind == "big"
