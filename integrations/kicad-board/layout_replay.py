"""Repeat a routed multi-footprint block through the command interface and apply a scoped ECO."""

import json
from pathlib import Path
import subprocess
import sys
import pcbnew as k
from electra_pcb.project import Project, identity


def verify_layout(directory, source, reader, checker, library, capture_map):
    dest = directory / "panel.kicad_pcb"
    bindings = directory / "panel.json"

    def cli(*args, check=True):
        return subprocess.run(
            [sys.executable, "-m", "electra_pcb", dest, bindings, *map(str, args)],
            cwd=Path(__file__).parent,
            text=True,
            capture_output=True,
            check=check,
        )

    cli("init", "--layers", 4, "--width-nm", 95_000_000, "--height-nm", 65_000_000)
    edits = directory / "edits.json"

    def apply(actions):
        edits.write_text(json.dumps(actions))
        cli("edit", edits, "--pin-table", reader)

    apply(
        [
            {
                "op": "place",
                "library": str(library),
                "name": "Dual",
                "reference": "LOCK",
                "position": [85_000_000, 45_000_000],
                "locked": True,
            }
        ]
    )

    def block(prefix, x):
        return {
            "op": "layout",
            "board": str(source),
            "offset": [x, 0],
            "references": {r: prefix + r for r in ["J1", "J7", "J3"]},
            "nets": {"N1": "N1" if x == 0 else "N11", "N2": "N2" if x == 0 else "N12"},
        }

    apply([block("A", 0), block("B", 42_000_000)])
    for index, name in [(1, "AJ1"), (2, "AJ7")]:
        cli("bind", index, name, "1=1", "2=2")
    cli("rule", 17, "1.1", "1.2", 23_000_000, 500_000)
    board = Project.open(dest, bindings)
    assert len(list(board.board.GetFootprints())) == 7

    def copper(b):
        return sorted(
            (
                t.GetNetname(),
                t.GetLayer(),
                tuple(t.GetStart()),
                tuple(t.GetEnd()),
                isinstance(t, k.PCB_VIA),
            )
            for t in b.GetTracks()
        )

    template = k.LoadBoard(str(source))
    physical = copper(template)
    shifted = [
        (
            "N11" if net == "N1" else "N12",
            layer,
            (a[0] + 42_000_000, a[1]),
            (b[0] + 42_000_000, b[1]),
            via,
        )
        for net, layer, a, b, via in physical
    ]
    assert copper(board.board) == sorted(physical + shifted)
    assert len(list(board.board.Zones())) == 2 * len(list(template.Zones()))
    ids = [identity(t) for t in board.board.GetTracks()] + [
        identity(z) for z in board.board.Zones()
    ]
    for fp in board.board.GetFootprints():
        ids.extend([identity(fp), *(identity(p) for p in fp.Pads())])
    assert len(set(ids)) == len(ids)

    def untouched(b):
        return sorted(
            (
                identity(f),
                f.GetReference(),
                tuple(f.GetPosition()),
                f.IsLocked(),
                [(identity(p), p.GetNetname()) for p in f.Pads()],
            )
            for f in b.GetFootprints()
            if f.GetReference().startswith("B") or f.GetReference() == "LOCK"
        )

    before = untouched(board.board)
    assert any(r[1] == "LOCK" and r[3] for r in before)
    pad = board.highlight((2, 1))[0]
    eco = {
        "op": "eco",
        "schematic": str(directory / "project.elcp"),
        "references": {"2": "AOUT"},
    }
    apply([eco, {"op": "undo-eco"}])
    assert json.loads(cli("probe", "2", "1").stdout)["reference"] == "AJ7"
    apply([eco])
    again = Project.open(dest, bindings)
    assert untouched(again.board) == before and again.parts[2].GetReference() == "AOUT"
    assert again.highlight((2, 1)) == [pad] and again.reverse[pad] == (2, 1)
    assert json.loads(cli("probe", "2", "1").stdout)["net"] == f"N{capture_map[2,1]}"
    report = json.loads(
        cli(
            "check",
            "--pair-binary",
            checker,
            "--copper-binary",
            Path(checker).with_name("electra-copper"),
        ).stdout
    )
    assert report["pair_margins_nm"] == [[17, 20000000, 20000000, 3000000, 500000]]
    print(
        "Two routed layout copies retained independent nets, new UUIDs, vias and zones after save and scoped ECO."
    )
