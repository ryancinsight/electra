"""Linear feedback fixtures with rational coefficients and SPICE card encodings."""
from fractions import Fraction
from random import Random


def fixture(seed):
    rng = Random(seed)
    parts = []
    inputs = {}

    def add(name, a, b, value):
        if rng.choice([False, True]):
            a, b = b, a
        parts.append((name, a, b, value))

    add("Vin", 1, 0, rng.choice([-3, 2, 4]))
    add("Vprobe", 2, 3, 0)
    add("Ibias", 2, 0, Fraction(rng.choice([-1, 1]), 8))
    for n in range(1, 8):
        add(f"R{n}", n, 0, rng.choice([4, 8, 16]))
    add("Rf", 4, 2, 16)
    add("Rreturn", 7, 3, 32)
    add("Eamp", 4, 0, Fraction(rng.choice([-3, -1, 1, 3]), 4))
    add("Gout", 5, 0, Fraction(rng.choice([-3, -1, 1, 3]), 64))
    add("Fmirror", 6, 0, Fraction(rng.choice([-3, -1, 1, 3]), 4))
    add("Hout", 7, 0, rng.choice([-2, -1, 1, 2]))
    inputs["Eamp"] = tuple(rng.sample([1, 2, 5], 2))
    inputs["Gout"] = tuple(rng.sample([2, 4, 6], 2))
    inputs["Fmirror"] = rng.choice(["Vprobe", "Eamp", "Hout"])
    inputs["Hout"] = rng.choice(["Vprobe", "Eamp"])
    return parts, inputs


def deck(parts, inputs, hierarchical):
    def card(name, a, b, value):
        control = inputs.get(name)
        fields = list(map(str, control)) if isinstance(control, tuple) else [control] if control else []
        return " ".join([name, str(a), str(b), *fields, value])

    # All fixture fractions are dyadic and exactly represented by these short decimals.
    if not hierarchical:
        return "\n".join(card(n, a, b, str(float(v))) for n, a, b, v in parts)
    pins = " ".join(str(i) for i in range(1, 8))
    units = {"E": "x", "G": "S", "F": "x", "H": "Ohm"}
    gains = [(n, v) for n, _, _, v in parts if n in inputs]
    overrides = " ".join(f"{n}={float(v)}" for n, v in gains)
    formals = " ".join(f"{n}=1{units[n[0]]}" for n, _ in gains)
    lines = [f"Xboard {pins} board", f".subckt board {pins}",
             f"Xcell {pins} cell {overrides}", ".ends", f".subckt cell {pins} params: {formals}"]
    lines.extend(card(n, a, b, f"{{{n}}}" if n in inputs else str(float(v))) for n, a, b, v in parts)
    return "\n".join([*lines, ".ends"])
