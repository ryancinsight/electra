"""Profile the 101-point floating-source sweep using an optimized qualification binary."""
import argparse
from pathlib import Path
import re
from statistics import median
import subprocess
from tempfile import TemporaryDirectory


def profile(binary, generator, counts, repetitions):
    with TemporaryDirectory() as temp:
        path = Path(temp) / "cells.cir"
        for cells in counts:
            path.write_bytes(subprocess.check_output([generator, str(cells * 4), "deck"]))
            for solver in ("direct", "dispatch"):
                command = [binary, "profile", path, "--solver", solver]
                warmup = subprocess.run(command, text=True, capture_output=True)
                if warmup.returncode:
                    if "DirectOrder" not in warmup.stderr:
                        warmup.check_returncode()
                    print(cells * 5, solver, warmup.stderr.strip())
                    continue
                observations = []
                for _ in range(repetitions):
                    out = subprocess.check_output(command, text=True)
                    values = dict((k, int(v)) for k, v in re.findall(r"(\w+)=(\d+)", out))
                    assert values.pop("points") == 101
                    observations.append(values)
                print(cells * 5, solver, {k: median(row[k] for row in observations) for k in observations[0]})


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--generator", type=Path, default=Path("target/release/examples/mna_profile"))
    parser.add_argument("--cells", nargs="+", type=int, default=[5, 25, 51, 250, 2500])
    parser.add_argument("--repetitions", type=int, default=9)
    args = parser.parse_args()
    if min(*args.cells, args.repetitions) < 1:
        parser.error("cell counts and repetitions must be positive")
    profile(args.binary.resolve(), args.generator.resolve(), args.cells, args.repetitions)
