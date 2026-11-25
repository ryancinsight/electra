"""Verify both transformer CLI models using the two winding voltage equations."""
import argparse
import csv
import io
import math
from decimal import Decimal as D
from pathlib import Path
import subprocess

from nodal import TAU, eliminate


def scalar(text):
    return D.from_float(float(text))


def windings(hz, lp, ls, k, load):
    # Unknowns: real and imaginary primary and secondary currents, entering the dots.
    w = TAU * scalar(hz)
    xp, xs, xm = w*lp, w*ls, w*k*(lp*ls).sqrt()
    matrix = [[0, -xp, 0, -xm, 1], [xp, 0, xm, 0, 0]]
    if load is None:
        matrix += [[0, 0, 1, 0, 0], [0, 0, 0, 1, 0]]
    else:
        matrix += [[0, -xm, load, -xs, 0], [xm, 0, xs, load, 0]]
    matrix = [list(map(D, row)) for row in matrix]
    eliminate(matrix)
    pr, pi, sr, si = (row[-1] for row in matrix)
    norm = pr*pr + pi*pi
    vr, vi = (-xm*pi-xs*si, xm*pr+xs*sr) if load is None else (-load*sr, -load*si)
    return vr, vi, pr/norm, -pi/norm


def rows(binary, mode, args, start, stop, count):
    data = subprocess.check_output([binary, mode, str(start), str(stop), str(count), *args], text=True)
    reader = csv.DictReader(io.StringIO(data))
    assert reader.fieldnames == ["frequency_Hz", "gain_re", "gain_im", "input_port",
                                "zin_re_ohm", "zin_im_ohm"]
    result = list(reader)
    assert len(result) == count
    for i, row in enumerate(result):
        assert math.isclose(float(row["frequency_Hz"]),start*(stop/start)**(i/(count-1)),rel_tol=5e-14)
    return result


def check_pair(row, fields, want):
    scale = max(map(abs, want))
    errors = []
    for field, ref in zip(fields, want):
        got = D(row[field])
        # Permit roundoff in a theoretically zero quadrature of the decimal solve.
        if abs(ref) <= scale * D("1e-45"):
            assert got.is_finite() and abs(got-ref) <= scale * D("1e-40"), (field, got, ref)
        else:
            error = abs(got/ref - 1)
            assert got.is_finite() and error <= D("1e-12"), (row, ref, error)
            errors.append(error)
    return max(errors, default=D(0))


def verify(binary):
    count = ideal_count = 0
    peak = D(0)
    for lp, ls in [("0.001","0.004"), ("0.16","0.01"), ("3.3e-6","0.0017")]:
        for k in ["0", "0.25", "-0.875", "0.9999999999999999", "1", "-1"]:
            for r in ["open", "0", "0.1", "47", "10000"]:
                if r == "0" and abs(float(k)) == 1:
                    continue
                for start, stop in [(1,1e8), (1e8,1)]:
                    for row in rows(binary, "coupled", [lp,ls,k,r], start, stop, 9):
                        assert row["input_port"] == "finite"
                        want = windings(row["frequency_Hz"], scalar(lp), scalar(ls), scalar(k),
                                        None if r == "open" else scalar(r))
                        peak = max(peak, check_pair(row, ("gain_re","gain_im"), want[:2]),
                                   check_pair(row, ("zin_re_ohm","zin_im_ohm"), want[2:]))
                        count += 1
    for ratio in ["-4", "0.5", "1", "10"]:
        for r in ["open","0.1","47","10000"]:
            for row in rows(binary, "ideal", [ratio,r], 1, 1e8, 9):
                peak = max(peak,check_pair(row, ("gain_re","gain_im"), (scalar(ratio), D(0))))
                if r == "open":
                    assert row["input_port"] == "open" and row["zin_re_ohm"] == row["zin_im_ohm"] == ""
                else:
                    assert row["input_port"] == "finite"
                    peak = max(peak,check_pair(row, ("zin_re_ohm","zin_im_ohm"),
                                               (scalar(r)/scalar(ratio)**2, D(0))))
                ideal_count += 1
    print(f"{count} coupled and {ideal_count} ideal samples, maximum quadrature relative error {peak:.3e}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary",type=Path)
    verify(parser.parse_args().binary.resolve())
