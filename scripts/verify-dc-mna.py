"""Check Electra's CLI against exact rational modified nodal analysis on small circuits."""

from argparse import ArgumentParser
from fractions import Fraction
from pathlib import Path
from random import Random
from tempfile import TemporaryDirectory
import re
import subprocess


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


def reference(parts):
    # Unknowns are non-reference node potentials followed by ideal-source currents.
    nodes = max(max(a, b) for _, a, b, _ in parts)
    sources = [name for name, *_ in parts if name.startswith("V")]
    slots = {name: nodes + i for i, name in enumerate(sources)}
    n = nodes + len(sources)
    matrix = [[Fraction(0) for _ in range(n + 1)] for _ in range(n)]
    for name, a, b, value in parts:
        ports = [(node - 1, sign) for node, sign in [(a, 1), (b, -1)] if node]
        if name.startswith("R"):
            for row, s in ports:
                for col, t in ports:
                    matrix[row][col] += Fraction(s*t, value)
        elif name.startswith("I"):
            for row, sign in ports:
                matrix[row][-1] -= sign*value
        else:
            k = slots[name]
            for row, sign in ports:
                matrix[row][k] += sign
                matrix[k][row] += sign
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


def verify(binary, label, options, absolute, hierarchical=False):
    with TemporaryDirectory() as tmp:
        path = Path(tmp) / "source-tree.cir"
        maximum = {"V": 0.0, "I": 0.0}
        count = 0
        outside_gate = 0
        for seed in range(32):
            parts = fixture(seed)
            expected = reference(parts)
            if hierarchical:
                expected = {(kind, f"X{name}/Xinner/{name[0]}part" if kind == "I" else name): value
                            for (kind, name), value in expected.items()}
            for order in (parts, list(reversed(parts))):
                deck = (hierarchical_deck(order) if hierarchical else
                        "".join(f"{name} {a} {b} {v}\n" for name, a, b, v in order))
                path.write_text(deck)
                result = subprocess.check_output([binary, "op", path, *options], text=True)
                probes = {(kind, name): float(value) for kind, name, value in
                          re.findall(r"^([IV])\(([^)]+)\) = (\S+) [VA]$", result, re.M)}
                if probes.keys() != expected.keys():
                    raise AssertionError(f"seed {seed}: probe inventory mismatch")
                for (kind, name), want in expected.items():
                    error = abs(probes[kind, name] - float(want))
                    maximum[kind] = max(maximum[kind], error)
                    relative = 1e-9 * abs(float(want))
                    gate = (1e-10 if kind == "V" else 1e-12) + relative
                    outside_gate += int(error > gate)
                    limit = absolute[kind] + relative
                    if error > limit:
                        raise AssertionError(f"seed {seed}, {kind}({name}): {error} exceeds {limit}")
                count += 1
        print(f"{label}: {count} decks, largest errors {maximum['V']:.3e} V and {maximum['I']:.3e} A, {outside_gate} probes outside the 0.2 fixture gate")


if __name__ == "__main__":
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("binary", nargs="?", type=Path, default=Path("target/release/electra"))
    parser.add_argument("--hierarchical", action="store_true",
                        help="represent each device as a nested instance with a typed override")
    args = parser.parse_args()
    layout = "hierarchical" if args.hierarchical else "flat"
    verify(args.binary.resolve(), f"{layout} default", [], {"V": 2e-8, "I": 2e-9}, args.hierarchical)
    verify(args.binary.resolve(), f"{layout} qualification", ["--atol-amps", "1e-14", "--rtol", "1e-13"],
           {"V": 1e-10, "I": 1e-12}, args.hierarchical)
