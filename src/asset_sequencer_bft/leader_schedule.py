from typing import Iterable, Iterator, List


def round_robin(validators: List[str]) -> Iterator[str]:
    """Yield validator ids in a deterministic round-robin schedule."""
    if not validators:
        raise ValueError("No validators provided")
    i = 0
    while True:
        yield validators[i % len(validators)]
        i += 1
