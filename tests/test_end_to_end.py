import os
import sys

# Ensure src is on the path when running tests from GitHub Actions
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from src.curvescript import parse
from src.compiler import compile_program
from src.curvevm import CurveVM


def test_buy_pipeline():
    script = "BUY 5"
    ast = parse(script)
    program = compile_program(ast)
    vm = CurveVM()
    vm.execute(program)
    assert vm.balance == 5
