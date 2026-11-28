"""Replay the two pinned heater references with NIST FiPy 4.0.3 on their declared grid."""
import argparse
from pathlib import Path

import fipy
import numpy as np
from fipy import CellVariable, DiffusionTerm, Grid2D, ImplicitSourceTerm, LinearLUSolver


def reference(path):
    records = [line.partition("#")[0].split() for line in path.read_text(encoding="utf-8").splitlines()]

    def values(name):
        return next(row[1:] for row in records if row and row[0] == name)

    width, height, rows, columns = values("geometry_mm")
    rows, columns = int(rows), int(columns)
    dx, dy = float(width) * 1e-3 / columns, float(height) * 1e-3 / rows
    mesh = Grid2D(nx=columns, ny=rows, dx=dx, dy=dy)
    source = np.zeros((rows, columns))
    for row in records:
        if row and row[0] == "heater":
            source[int(row[1]), int(row[2])] = float(row[3])
    power = float(values("input_power_w")[0])
    ambient = float(values("ambient_c")[0])
    h = float(values("air_conductance_w_m2k")[0])
    sheet = sum(float(row[2]) * 1e-3 * float(row[3]) for row in records if row and row[0] == "layer_mm_w_mk")
    flux = CellVariable(mesh=mesh, value=source.ravel() * power / (dx * dy))
    rise = CellVariable(mesh=mesh, value=0.)
    equation = DiffusionTerm(coeff=sheet) - ImplicitSourceTerm(coeff=h) + flux == 0
    solver = LinearLUSolver()
    equation.solve(var=rise, solver=solver)
    residual = np.max(np.abs(equation.justResidualVector(var=rise, solver=solver)))
    if not residual <= 1e-12:
        raise ValueError(f"FiPy nodal residual {residual} W exceeds the reference budget")
    reference_rows = []
    sensors = list(zip(mesh.cellCenters[0].value, mesh.cellCenters[1].value, rise.value, strict=True))
    for index, (x, y, value) in enumerate(sensors):
        reference_rows.append(f"sensor_c cell-{index} {x * 1000:.12f} {y * 1000:.12f} {ambient + value:.6f} 0.000001")
    x, y, value = max(sensors, key=lambda point: point[2])
    reference_rows.append(f"peak_c {x * 1000:.12f} {y * 1000:.12f} {ambient + value:.6f} 0.000001")
    return reference_rows


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    fixture_dir = Path(__file__).resolve().parents[1] / "crates/electra-board-field/examples/heater_coupon/fixtures"
    defaults = [fixture_dir / f"hannover_{layout}_fipy.txt" for layout in ("hilbert", "meander")]
    parser.add_argument("coupon", nargs="*", type=Path, default=defaults)
    args = parser.parse_args()
    if fipy.__version__ != "4.0.3":
        parser.error("the fixture references FiPy 4.0.3")
    for coupon in args.coupon:
        expected = reference(coupon)
        recorded = [line for line in coupon.read_text(encoding="utf-8").splitlines()
                    if line.startswith(("sensor_c ", "peak_c "))]
        print(coupon.name, *expected, sep="\n")
        if expected != recorded:
            raise SystemExit(f"{coupon}: recorded reference rows differ from FiPy")
