"""Qualify the public noise CLI by a reciprocal 1 A port injection and nodal power sum."""
import argparse
import csv
import io
import math
import subprocess
from decimal import Decimal as D
from pathlib import Path
from random import Random

from nodal import solve_ladder


def reference(stages, frequency):
    _, voltages = solve_ladder([s[:3] for s in stages], frequency)
    prev_re = prev_im = total = D(0)
    for (r, _, g, tr, tg), (re, im) in zip(stages, voltages):
        # The 1 A reciprocal test source turns each voltage into a transimpedance.
        drop2 = (re - prev_re)**2 + (im - prev_im)**2
        total += D(tr) * drop2 / D(r) + D(tg) * D(g) * (re**2 + im**2)
        prev_re, prev_im = re, im
    return 4 * D("1.380649e-23") * total


def verify(binary):
    rng = Random(3603)
    peak = 0.
    points = 0
    for n in range(1, 9):
        for distribution in ("equilibrium", "mixed", "series", "shunt", "cold"):
            stages = []
            for _ in range(n):
                tr, tg = 300, 300
                if distribution != "equilibrium":
                    tr = rng.randrange(75, 501) if distribution in ("mixed", "series") else 0
                    tg = rng.randrange(75, 501) if distribution in ("mixed", "shunt") else 0
                stages.append((rng.choice(["22", "100", "8200"]), rng.choice(["0", "1e-9", "2.2e-6"]),
                               rng.choice(["0", "3.3e-4", "0.01"]), str(tr), str(tg)))
            for start, stop in [(1, 1e7), (1e7, 1)]:
                output = subprocess.check_output([binary, "--noise", str(start), str(stop), "9",
                                                  *(v for s in stages for v in s)], text=True)
                reader = csv.DictReader(io.StringIO(output))
                assert reader.fieldnames == ["frequency_Hz", "output_voltage_psd_V2_per_Hz"]
                rows = list(reader)
                assert len(rows) == 9
                for i, row in enumerate(rows):
                    f = D(row["frequency_Hz"])
                    assert math.isclose(float(f), start*(stop/start)**(i/8), rel_tol=5e-14)
                    want = reference(stages, f)
                    got = D(row["output_voltage_psd_V2_per_Hz"])
                    error = abs(got/want - 1) if want else abs(got)
                    if not got.is_finite() or got < 0 or error > (D("1e-12") if want else 0):
                        raise AssertionError((stages, f, got, want))
                    peak = max(peak, float(error))
                    points += 1
    print(f"{points} noise CLI samples, maximum relative error {peak:.3e}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    verify(parser.parse_args().binary.resolve())
