import os
import sys
import base64
import json
import hashlib

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from src.curvescript import parse
from src.compiler import compile_program
from src.rollup import FakeSolanaClient, BatchPoster
from src.asset_sequencer_bft import Consensus, Block, round_robin


def test_round_robin_schedule():
    vals = ["A", "B", "C"]
    sched = round_robin(vals)
    assert [next(sched) for _ in range(5)] == ["A", "B", "C", "A", "B"]


def test_consensus_commit():
    script = "BUY 1"
    program = compile_program(parse(script))

    client = FakeSolanaClient()
    poster = BatchPoster(client)
    consensus = Consensus(["A", "B", "C"], poster)
    block = Block(program=program)

    tx = consensus.propose_and_commit(block)
    assert tx == client.sent[-1]
    payload = json.loads(base64.b64decode(tx).decode("utf-8"))
    encoded = json.dumps(payload["program"], sort_keys=True).encode("utf-8")
    expected_root = hashlib.sha256(encoded).hexdigest()
    assert payload["root"] == expected_root
