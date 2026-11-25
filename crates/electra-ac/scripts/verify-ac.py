"""Compare the public CLI with 70-digit, independently assembled nodal equations."""
import argparse
import cmath
import csv
from decimal import Decimal as D, getcontext
import io
import math
from pathlib import Path
from random import Random
import subprocess

getcontext().prec = 70
TAU = D("6.2831853071795864769252867665590057683943387987502116419498891846156328")


def reference(stages, frequency):
    # Real block representation of G + jB. Two RHS columns solve independent
    # experiments: a 1 V source for H, and a 1 A output injection for Zout.
    n = len(stages)
    size = 2 * n
    matrix = [[D(0) for _ in range(size + 2)] for _ in range(size)]
    for j, (r, c, load) in enumerate(stages):
        g = 1 / D(r)
        b = TAU * frequency * D(c)
        for offset in (0, n):
            row = offset + j
            matrix[row][row] += g + D(load)
            if j:
                matrix[row - 1][row - 1] += g
                matrix[row][row - 1] -= g
                matrix[row - 1][row] -= g
        matrix[j][j + n] -= b
        matrix[j + n][j] += b
    matrix[0][-2] = 1 / D(stages[0][0])
    matrix[n - 1][-1] = 1
    for j in range(size):
        k = max(range(j, size), key=lambda i: abs(matrix[i][j]))
        matrix[j], matrix[k] = matrix[k], matrix[j]
        pivot = matrix[j][j]
        matrix[j] = [x / pivot for x in matrix[j]]
        for i, row in enumerate(matrix):
            if i != j and row[j]:
                factor = row[j]
                matrix[i] = [x - factor * y for x, y in zip(row, matrix[j])]
    return tuple(complex(float(matrix[n - 1][col]), float(matrix[size - 1][col]))
                 for col in (-2, -1))


def verify(binary):
    errors = dict(gain=0., impedance=0., magnitude=0., phase_deg=0.)
    points = 0
    for seed in range(32):
        rng = Random(seed)
        stages = [(rng.choice(["10", "47", "220", "1300", "8200"]),
                   rng.choice(["0", "470e-12", "8.2e-9", "0.22e-6", "1e-6"]),
                   rng.choice(["0", "0", "1e-5", "0.00033"])) for _ in range(1 + seed % 8)]
        start, stop = (10, 1e7) if seed % 2 == 0 else (1e7, 10)
        output = subprocess.check_output([binary, str(start), str(stop), "13",
                                         *(v for stage in stages for v in stage)], text=True)
        rows = list(csv.DictReader(io.StringIO(output)))
        assert len(rows) == 13
        for idx, row in enumerate(rows):
            f = D(row["frequency_Hz"])
            grid = start * (stop/start) ** (idx/12)
            assert math.isclose(float(f), grid, rel_tol=5e-14)
            h, z = reference(stages, f)
            got_h = complex(float(row["gain_re"]), float(row["gain_im"]))
            got_z = complex(float(row["zout_re_ohm"]), float(row["zout_im_ohm"]))
            e = {"gain": abs(got_h-h)/abs(h), "impedance": abs(got_z-z)/abs(z),
                 "magnitude": abs(float(row["gain_magnitude"])/abs(h)-1),
                 "phase_deg": abs(math.remainder(float(row["phase_deg"]) - math.degrees(cmath.phase(h)), 360))}
            for name, value in e.items():
                limit = 1e-10 if name == "phase_deg" else 5e-13
                if not math.isfinite(value) or value > limit:
                    raise AssertionError((seed, f, name, value, limit))
                errors[name] = max(errors[name], value)
            points += 1
    print(f"{points} CLI samples, {2*points} complex comparisons; maxima {errors}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    verify(parser.parse_args().binary.resolve())
