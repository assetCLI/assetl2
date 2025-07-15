import os
import sys

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
    # Ensure commit returned the base64-encoded payload
    assert tx_sig == client.sent[-1]
