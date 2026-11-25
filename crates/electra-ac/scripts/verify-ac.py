"""Compare the public CLI with 70-digit, independently assembled nodal equations."""
import argparse
import cmath
import csv
from decimal import Decimal as D
import io
import math
from pathlib import Path
from random import Random
import subprocess

from nodal import solve_ladder


def reference(stages, frequency):
    return tuple(complex(float(re), float(im)) for re, im in
                 (voltages[-1] for voltages in solve_ladder(stages, frequency)))


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
