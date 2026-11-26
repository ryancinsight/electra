"""Compare release AC node voltages against an independent complex incidence-matrix solve."""
import argparse
import csv
import io
from pathlib import Path
import random
import statistics
import subprocess
import tempfile
from time import perf_counter

import numpy as np


def check(binary, path, n):
    rng = random.Random(n)
    # The drive node is index n+1, ground is zero. A resistor chain anchors every node.
    edges = [("R", k, k + 1, rng.uniform(10, 100)) for k in range(n + 1)]
    for k in range(1, n + 1):
        edges.append(("C", k, 0, rng.uniform(1e-8, 1e-6)))
        edges.append(("L", k, rng.randrange(k), rng.uniform(1e-3, 0.05)))
    lines = [f"Vdrive {n+1} 0 AC 1 90"]
    incidence = np.zeros((len(edges), n + 2))
    admittance = np.zeros(len(edges), dtype=complex)
    omega = 2 * np.pi * 1000
    for i, (kind, a, b, value) in enumerate(edges):
        lines.append(f"{kind}{i} {a} {b} {value!r}")
        incidence[i, a], incidence[i, b] = 1, -1
        admittance[i] = {"R": 1 / value, "C": 1j * omega * value, "L": 1 / (1j * omega * value)}[kind]
    path.write_text("\n".join(lines))
    matrix = incidence.T @ (admittance[:, None] * incidence)
    expected = np.linalg.solve(matrix[1:-1, 1:-1], -1j * matrix[1:-1, -1])
    times = []
    for _ in range(7):
        start = perf_counter()
        proc = subprocess.run([binary, path, "1000"], check=True, capture_output=True, text=True)
        times.append((perf_counter() - start) * 1000)
    rows = csv.DictReader(io.StringIO(proc.stdout))
    actual = {int(r["node"]): complex(float(r["real_V"]), float(r["imag_V"])) for r in rows}
    computed = [actual[k] for k in range(1, n + 1)]
    np.testing.assert_allclose(computed, expected, atol=1e-12, rtol=1e-10)
    return statistics.median(times), np.max(np.abs(computed - expected)), proc.stderr.strip()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as tmp:
        for nodes in [4, 32, 128]:
            print(nodes, *check(args.binary.resolve(), Path(tmp) / "rlc.cir", nodes))
