import os
import sys
from unittest.mock import MagicMock

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from src.compiler import Instruction
from src.mempool import Mempool, Tx
from src.miner import Miner
from src.rollup import FakeSolanaClient, BatchPoster
from src.asset_sequencer_bft import Consensus


def test_miner_mines_block():
    mp = Mempool()
    program = [Instruction("BUY", 1)]
    mp.add_tx(Tx(sender="A", nonce=0, program=program))

    client = FakeSolanaClient()
    poster = BatchPoster(client)
    consensus = Consensus(["A"], poster)
    consensus.propose_and_commit = MagicMock(return_value="sig")

    miner = Miner(mp, consensus)
    sig = miner.mine("fast", 1)
    assert sig == "sig"
    consensus.propose_and_commit.assert_called_once()
    block_arg = consensus.propose_and_commit.call_args[0][0]
    assert block_arg.kind == "fast"
