import os
import sys
import json
import base64
import hashlib

# Ensure src is on the path when running tests from GitHub Actions
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from src.curvescript import parse
from src.compiler import compile_program
from src.curvevm import CurveVM
from src.rollup import BatchPoster, FakeSolanaClient


def test_buy_pipeline_and_post():
    script = "BUY 5"
    ast = parse(script)
    program = compile_program(ast)
    vm = CurveVM()
    vm.execute(program)
    assert vm.balance == 5

    client = FakeSolanaClient()
    poster = BatchPoster(client)
    tx_sig = poster.commit(program)
    assert tx_sig == client.sent[-1]
    payload = json.loads(base64.b64decode(tx_sig).decode("utf-8"))
    encoded = json.dumps(payload["program"], sort_keys=True).encode("utf-8")
    expected_root = hashlib.sha256(encoded).hexdigest()
    assert payload["root"] == expected_root


def test_all_opcodes_pipeline():
    script = """BUY 5\nSELL 2\nADD_LIQUIDITY 3\nMIGRATE_TO_AMM 1"""
    ast = parse(script)
    program = compile_program(ast)
    vm = CurveVM()
    vm.execute(program)
    assert vm.balance == 3
    assert vm.liquidity == 3
    assert vm.migrated_to_amm is True

    client = FakeSolanaClient()
    poster = BatchPoster(client)
    tx_sig = poster.commit(program)
    assert tx_sig == client.sent[-1]
    payload = json.loads(base64.b64decode(tx_sig).decode("utf-8"))
    encoded = json.dumps(payload["program"], sort_keys=True).encode("utf-8")
    expected_root = hashlib.sha256(encoded).hexdigest()
    assert payload["root"] == expected_root

