"""Check Electra's CLI against exact rational modified nodal analysis on small circuits."""

from argparse import ArgumentParser
from fractions import Fraction
from math import isfinite
from pathlib import Path
from random import Random
from tempfile import TemporaryDirectory
import re
import subprocess

import controlled_fixtures


def fixture(seed):
    rng = Random(seed)
    parts = []

    def add(kind, a, b, value):
        if rng.choice([True, False]):
            a, b = b, a
        parts.append((f"{kind}{len(parts)}", a, b, value))

    # A source tree in each group; the first group contains the reference.
    for group in [(0, 1, 2), (3, 4, 5), (6, 7)]:
        for i in range(1, len(group)):
            add("V", group[i], rng.choice(group[:i]), rng.randint(-6, 6))
    for node in range(1, 9):
        add("R", node, 0, rng.choice([4, 8, 16]))
    for _ in range(10):
        a, b = rng.sample(range(9), 2)
        add("R", a, b, rng.choice([64, 128, 256]))
    for _ in range(5):
        a, b = rng.sample(range(9), 2)
        add("I", a, b, rng.randint(-2, 2))
    return parts


def reference(parts, controls):
    # Unknowns are non-reference node potentials followed by ideal-source currents.
    nodes = max(max(a, b) for _, a, b, _ in parts)
    sources = [name for name, *_ in parts if name[0] in "VEH"]
    slots = {name: nodes + i for i, name in enumerate(sources)}
    n = nodes + len(sources)
    matrix = [[Fraction(0) for _ in range(n + 1)] for _ in range(n)]
    def terminals(a, b):
        return [(node - 1, sign) for node, sign in [(a, 1), (b, -1)] if node]

    def control_terms(name):
        signal = controls[name]
        return terminals(*signal) if isinstance(signal, tuple) else [(slots[signal], 1)]

    for name, a, b, value in parts:
        ports = terminals(a, b)
        if name.startswith("R"):
            for row, s in ports:
                for col, t in ports:
                    matrix[row][col] += Fraction(s*t, value)
        elif name.startswith("I"):
            for row, sign in ports:
                matrix[row][-1] -= sign*value
        elif name[0] in "GF":
            for row, sign in ports:
                for col, orientation in control_terms(name):
                    matrix[row][col] += sign * orientation * value
        else:
            k = slots[name]
            for row, sign in ports:
                matrix[row][k] += sign
                matrix[k][row] += sign
            if name in controls:
                for col, orientation in control_terms(name):
                    matrix[k][col] -= orientation * value
            else:
                matrix[k][-1] = Fraction(value)

    for col in range(n):
        pivot = next(row for row in range(col, n) if matrix[row][col])
        matrix[col], matrix[pivot] = matrix[pivot], matrix[col]
        diagonal = matrix[col][col]
        matrix[col] = [x / diagonal for x in matrix[col]]
        for row in range(n):
            factor = matrix[row][col]
            if row != col and factor:
                matrix[row] = [x - factor*y for x, y in zip(matrix[row], matrix[col])]
    solution = [row[-1] for row in matrix]
    volts = [Fraction(0), *solution[:nodes]]
    expected = {("V", str(i)): v for i, v in enumerate(volts)}
    for name, a, b, value in parts:
        if name.startswith("R"):
            current = (volts[a] - volts[b]) / value
        elif name.startswith("I"):
            current = value
        elif name[0] in "GF":
            current = value * sum(s * solution[col] for col, s in control_terms(name))
        else:
            current = solution[slots[name]]
        expected[("I", name)] = current
    return expected


def hierarchical_deck(parts):
    # Reverse the pin order at the inner boundary. The physical polarity is
    # determined by the leaf formal bindings, independently of card ordering.
    lines = [f"X{name} {a} {b} pair{name[0]} value={value}"
             for name, a, b, value in parts]
    for kind, unit in (("V", "V"), ("I", "A"), ("R", "Ohm")):
        lines.extend([
            f".subckt pair{kind} a b params: value=1{unit}",
            f"Xinner b a leaf{kind} quantity={{value}}",
            ".ends",
            f".subckt leaf{kind} minus plus params: quantity=1{unit}",
            f"{kind}part plus minus {{quantity}}",
            ".ends",
        ])
    return "\n".join(lines)


def qualified(name, hierarchy, controlled):
    if hierarchy and controlled:
        return f"Xboard/Xcell/{name}"
    return f"X{name}/Xinner/{name[0]}part" if hierarchy else name


def check_probes(text, expected, hierarchy, controlled, absolute, maximum, context):
    probes = {}
    for kind, name, value, unit in re.findall(r"^([IV])\(([^)]+)\) = (\S+) (\S+)$", text, re.M):
        if unit != ("V" if kind == "V" else "A"):
            raise AssertionError(f"{context}: incorrect unit {unit} for {kind}({name})")
        key = (kind, name)
        if key in probes or not isfinite(float(value)):
            raise AssertionError(f"{context}: duplicate or nonfinite probe {key}")
        probes[key] = float(value)
    outside = 0
    for (kind, name), want in expected.items():
        key = (kind, qualified(name, hierarchy, controlled) if kind == "I" else name)
        error = abs(probes.pop(key) - float(want))
        maximum[kind] = max(maximum[kind], error)
        relative = 1e-9 * abs(float(want))
        outside += int(error > (1e-10 if kind == "V" else 1e-12) + relative)
        if error > absolute[kind] + relative:
            raise AssertionError(f"{context}: {key} error {error} exceeds {absolute[kind] + relative}")
    if probes:
        raise AssertionError(f"{context}: unexpected probes {list(probes)}")
    return outside


def verify(binary, label, options, absolute, hierarchy=False, sweep=False, controlled=False):
    maximum = {"V": 0.0, "I": 0.0}
    outside = count = comparisons = 0
    with TemporaryDirectory() as tmp:
        path = Path(tmp) / "source-tree.cir"
        for seed in range(32):
            parts, controls = controlled_fixtures.fixture(seed) if controlled else (fixture(seed), {})
            driven = next(name for name, *_ in parts if name[0] == ("V" if seed % 2 else "I"))
            source = qualified(driven, hierarchy, controlled)
            grid = list(range(-2, 3)) if seed % 4 < 2 else list(range(2, -3, -1))
            expected = ([reference([(name, a, b, q if name == driven else v)
                                    for name, a, b, v in parts], controls) for q in grid] if sweep
                        else [reference(parts, controls)])
            for order in (parts, list(reversed(parts))):
                deck = (controlled_fixtures.deck(order, controls, hierarchy) if controlled else
                        hierarchical_deck(order) if hierarchy else
                        "".join(f"{name} {a} {b} {v}\n" for name, a, b, v in order))
                if sweep:
                    deck += f"\n.dc {source} {grid[0]} {grid[-1]} {grid[1]-grid[0]}"
                path.write_text(deck)
                output = subprocess.check_output([binary, "dc" if sweep else "op", path, *options], text=True)
                samples = [output]
                if sweep:
                    unit = "V" if driven[0] == "V" else "A"
                    groups = re.split(rf"^{re.escape(source)} = (\S+) {unit}$", output, flags=re.M)
                    if groups[0].strip() or [float(q) for q in groups[1::2]] != grid:
                        raise AssertionError(f"seed {seed}: excitation sequence differs from {grid}")
                    samples = groups[2::2]
                for sample, oracle in zip(samples, expected, strict=True):
                    outside += check_probes(sample, oracle, hierarchy, controlled, absolute, maximum, f"seed {seed}")
                    comparisons += len(oracle)
                count += 1
    print(f"{label}: {count} decks, {comparisons} probe comparisons, largest errors "
          f"{maximum['V']:.3e} V and {maximum['I']:.3e} A, {outside} outside the 0.2 fixture gate")


if __name__ == "__main__":
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("binary", nargs="?", type=Path, default=Path("target/release/electra"))
    parser.add_argument("--hierarchical", action="store_true",
                        help="represent each device as a nested instance with a typed override")
    parser.add_argument("--solver", choices=["reduced", "mna", "direct", "dispatch", "sparse-first"], default="reduced")
    parser.add_argument("--sweep", action="store_true", help="verify five points per deck against separate exact solves")
    parser.add_argument("--controlled", action="store_true", help="test all four dependent-source families in coupled feedback networks")
    args = parser.parse_args()
    if args.controlled and args.solver == "reduced":
        parser.error("controlled devices require an MNA solver")
    solver_options = []
    if args.solver != "reduced":
        solver_options += ["--solver", args.solver, "--atol-volts", "1e-12"]
    layout = "hierarchical" if args.hierarchical else "flat"
    nominal_gate = {"V": 1e-10, "I": 1e-12} if args.solver != "reduced" else {"V": 2e-8, "I": 2e-9}
    verify(args.binary.resolve(), f"{args.solver} {layout} default", solver_options, nominal_gate, args.hierarchical, args.sweep, args.controlled)
    verify(args.binary.resolve(), f"{args.solver} {layout} qualification", [*solver_options, "--atol-amps", "1e-14", "--rtol", "1e-13"],
           {"V": 1e-10, "I": 1e-12}, args.hierarchical, args.sweep, args.controlled)
