"""Compare every node of seeded RLC trees or rings with SciPy's pivoted sparse solve."""
import argparse
import csv
import io
from pathlib import Path
import statistics
import subprocess
import tempfile
from time import perf_counter

import numpy as np
from scipy.sparse import coo_matrix
from scipy.sparse.linalg import spsolve


def check(binary, path, count, ring):
    rng = np.random.default_rng(29)
    omega = 2 * np.pi * 1000
    deck = ["Vdrive in 0 AC 1 90", "Rdrive in n0 1"]
    rows, cols, values = [0], [0], [1.0]
    for i in range(count):
        cap = rng.uniform(1e-7, 1e-6)
        deck.append(f"C{i} n{i} 0 {cap}")
        rows.append(i)
        cols.append(i)
        values.append(1j * omega * cap)
        if i:
            parent = i - 1 if ring else (i - 1) // 2
            kind, value = ("L", rng.uniform(1e-4, 1e-3)) if i % 3 == 0 else ("R", rng.uniform(1, 10))
            deck.append(f"{kind}{i} n{i} n{parent} {value}")
            y = 1 / (1j * omega * value) if kind == "L" else 1 / value
            rows.extend((parent, i, parent, i))
            cols.extend((parent, i, i, parent))
            values.extend((y, y, -y, -y))
    if ring:
        # Close the cycle and drive the far side through a second fixed terminal.
        end = count - 1
        deck.extend([f"Rclose n{end} n0 2", f"Vfar far 0 AC 1", f"Rfar far n{count // 2} 1"])
        rows.extend((end, 0, end, 0, count // 2))
        cols.extend((end, 0, 0, end, count // 2))
        values.extend((0.5, 0.5, -0.5, -0.5, 1.0))
    path.write_text("\n".join(deck))
    matrix = coo_matrix((values, (rows, cols)), shape=(count, count)).tocsc()
    rhs = np.zeros(count, dtype=complex)
    rhs[0] = 1j
    if ring:
        rhs[count // 2] += 1
    reference = spsolve(matrix, rhs)
    elapsed = []
    for _ in range(7):
        start = perf_counter()
        run = subprocess.run([binary, path, "1000"], check=True, capture_output=True, text=True)
        elapsed.append(1000 * (perf_counter() - start))
    actual = np.full(count, np.nan + 0j)
    for row in csv.DictReader(io.StringIO(run.stdout)):
        if row["node"].startswith("n"):
            actual[int(row["node"][1:])] = complex(float(row["real_V"]), float(row["imag_V"]))
    np.testing.assert_allclose(actual, reference, rtol=1e-10, atol=1e-12)
    print(count, statistics.median(elapsed), max(abs(actual - reference)), run.stderr.strip())


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--ring", action="store_true")
    args = parser.parse_args()
    binary = args.binary.resolve()
    with tempfile.TemporaryDirectory() as tmp:
        for n in (129, 1023, 8191):
            check(binary, Path(tmp) / "network.cir", n, args.ring)
