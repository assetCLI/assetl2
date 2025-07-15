import base64
import json
from typing import List
import hashlib

from .compiler import Instruction

class FakeSolanaClient:
    """A minimal stub representing the Solana RPC client."""
    def __init__(self) -> None:
        self.sent: List[str] = []

    def send_transaction(self, data: bytes) -> str:
        b64 = base64.b64encode(data).decode("utf-8")
        self.sent.append(b64)
        # In real usage this would return the transaction signature.
        return b64

class BatchPoster:
    """Posts batches of CurveVM instructions to the AssetRollup program."""
    def __init__(self, client: FakeSolanaClient) -> None:
        self.client = client

    def commit(self, program: List[Instruction]) -> str:
        """Serialize the program, compute a Merkle-style root and post it."""
        instr_list = [
            {"op": ins.opcode, "arg": ins.operand} for ins in program
        ]
        encoded = json.dumps(instr_list, sort_keys=True).encode("utf-8")
        root = hashlib.sha256(encoded).hexdigest()
        payload = json.dumps({"root": root, "program": instr_list}).encode("utf-8")
        # Send to Solana (stubbed)
        return self.client.send_transaction(payload)
