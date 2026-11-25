"""Classify sparse pivots on independently solved Electra DC fixtures."""
import argparse
from collections import Counter
from pathlib import Path
import runpy
import subprocess
import sys
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))
oracle = runpy.run_path(str(ROOT / "scripts/verify-dc-mna.py"))
controlled = oracle["controlled_fixtures"]


def solve(binary, path, provider):
    return subprocess.run([binary, "op", path, "--solver", provider], capture_output=True, text=True)


def check_default_dispatch(binary, path):
    fixture = ROOT / "integrations/leto-direct/fixtures/pivot.cir"
    path.write_text(fixture.read_text() + "".join(f"Rpad{i} p{i} 0 1\n" for i in range(27)))
    audit = solve(binary, path, "audit")
    audit.check_returncode()
    status = audit.stderr.strip()
    assert status.startswith("order=33 nnz=43 raw="), status
    result = solve(binary, path, "dispatch")
    if status.endswith("singular-pivot"):
        assert result.returncode and "singular" in result.stderr and "column 32" in result.stderr, result.stderr
    else:
        assert status.endswith("pivot-required"), status
        result.check_returncode()
        assert result.stdout == audit.stdout
    print("default dispatcher reproducer", status, "exit", result.returncode)


def check(binary):
    with TemporaryDirectory() as temp:
        path = Path(temp) / "case.cir"
        check_default_dispatch(binary, path)
        for family in ("independent", "controlled"):
            counts = Counter()
            for seed in range(32):
                parts, controls = controlled.fixture(seed) if family == "controlled" else (oracle["fixture"](seed), {})
                expected = oracle["reference"](parts, controls)
                for order in (parts, list(reversed(parts))):
                    path.write_text(controlled.deck(order, controls, False))
                    raw = solve(binary, path, "audit")
                    raw.check_returncode()
                    status = raw.stderr.strip().split("raw=")[1]
                    attempt = solve(binary, path, "sparse-first")
                    if status == "singular-pivot":
                        assert attempt.returncode and "singular" in attempt.stderr, attempt.stderr
                    else:
                        assert status in ("sparse", "pivot-required"), status
                        attempt.check_returncode()
                        oracle["check_probes"](attempt.stdout, expected, False, family == "controlled",
                                               {"V": 1e-10, "I": 1e-12}, {"V": 0., "I": 0.}, (family, seed))
                    counts[status] += 1
            print(family, dict(sorted(counts.items())))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    check(parser.parse_args().binary.resolve())
