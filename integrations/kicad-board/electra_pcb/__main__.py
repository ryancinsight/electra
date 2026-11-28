"""Edit a board from an Electra capture, query cross-highlights, or run sign-off."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

import pcbnew as k
from . import geometry
from .template import place_layout
from .project import Project, capture, identity
from .settings import set_class, write_json
from .check import measure, copper


def save(board, path):
    with tempfile.TemporaryDirectory(dir=path.parent) as directory:
        temporary = Path(directory) / path.name
        if not k.SaveBoard(str(temporary), board):
            raise OSError(f"could not save {path}")
        os.replace(temporary, path)


def run_edits(project, actions, reader):
    board = project.board
    for action in actions:
        op = action["op"]
        if op == "eco":
            refs = {int(i): name for i, name in action.get("references", {}).items()}
            project.apply(capture(reader, action["schematic"]), refs)
        elif op == "undo-eco":
            project.undo()
        elif op == "layout":
            path = Path(action["board"])
            if not path.is_file():
                raise FileNotFoundError(path)
            place_layout(
                board,
                k.LoadBoard(str(path)),
                action["offset"],
                action["references"],
                action["nets"],
            )
        elif op == "place":
            geometry.place(
                board,
                Path(action["library"]),
                action["name"],
                action["reference"],
                action["position"],
                locked=action.get("locked", False),
            )
        elif op in ("track", "via", "zone", "keepout"):
            net = None if op == "keepout" else board.FindNet(action["net"])
            if op != "keepout" and net is None:
                raise ValueError("unknown copper net")
            if op == "via":
                geometry.through_via(
                    board, net, action["position"], action["diameter"], action["drill"]
                )
                continue
            layer = board.GetLayerID(action["layer"])
            if layer not in board.GetEnabledLayers().CuStack():
                raise ValueError("requested layer is outside the enabled copper stack")
            if op == "track":
                geometry.segment(
                    board, net, layer, action["start"], action["end"], action["width"]
                )
            else:
                geometry.polygon(
                    board, layer, [tuple(p) for p in action["points"]], net=net
                )
        else:
            raise ValueError(f"unknown operation {op}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("board", type=Path)
    parser.add_argument("bindings", type=Path)
    subs = parser.add_subparsers(dest="op", required=True)
    init = subs.add_parser("init")
    init.add_argument("--layers", type=int, default=4)
    init.add_argument("--width-nm", type=int, required=True)
    init.add_argument("--height-nm", type=int, required=True)
    bind = subs.add_parser("bind")
    bind.add_argument("part", type=int)
    bind.add_argument("reference")
    bind.add_argument("pins", nargs="+", help="schematic_pin=physical_pad_number")
    rule = subs.add_parser("rule")
    rule.add_argument("id", type=int)
    rule.add_argument("positive", help="occurrence.pin")
    rule.add_argument("negative", help="occurrence.pin")
    rule.add_argument("max_length_nm", type=int)
    rule.add_argument("max_skew_nm", type=int)
    nc = subs.add_parser("net-class")
    nc.add_argument("name")
    nc.add_argument("width_nm", type=int)
    nc.add_argument("clearance_nm", type=int)
    nc.add_argument("nets", type=int, nargs="+")
    edit = subs.add_parser("edit")
    edit.add_argument("actions", type=Path)
    edit.add_argument("--pin-table", default="electra-pin-table")
    probe = subs.add_parser("probe")
    probe.add_argument("part", type=int)
    probe.add_argument("pin", type=int)
    reverse = subs.add_parser("locate")
    reverse.add_argument("uuid")
    check = subs.add_parser("check")
    check.add_argument("--pair-binary", default="electra-pair")
    check.add_argument("--copper-binary", default="electra-copper")
    args = parser.parse_args()
    if args.op == "init":
        if (
            args.layers not in range(2, 33, 2)
            or not 0 < min(args.width_nm, args.height_nm)
            or max(args.width_nm, args.height_nm) > 2**31 - 1
        ):
            raise ValueError("invalid board size or copper layer count")
        if args.board.exists() or args.bindings.exists():
            raise FileExistsError("board or binding document already exists")
        board = k.BOARD()
        board.SetCopperLayerCount(args.layers)
        edge = k.PCB_SHAPE(board)
        edge.SetShape(k.SHAPE_T_RECT)
        edge.SetLayer(k.Edge_Cuts)
        edge.SetStart(k.VECTOR2I(0, 0))
        edge.SetEnd(k.VECTOR2I(args.width_nm, args.height_nm))
        edge.SetWidth(50_000)
        board.Add(edge)
        save(board, args.board)
        try:
            write_json(
                args.bindings,
                {
                    "parts": {},
                    "rules": {},
                    "copper_limits_nm": [200000, 200000, 100000],
                },
            )
        except Exception:
            args.board.unlink()
            raise
        return 0
    if args.op == "net-class":
        set_class(
            args.board.with_suffix(".kicad_pro"),
            args.name,
            args.nets,
            args.width_nm,
            args.clearance_nm,
        )
        return 0
    project = Project.open(args.board, args.bindings)
    if args.op == "bind":
        candidates = [
            f
            for f in project.board.GetFootprints()
            if f.GetReference() == args.reference
        ]
        if len(candidates) != 1 or args.part < 0:
            raise ValueError("select one footprint and a valid occurrence")
        mappings = [p.split("=", 1) for p in args.pins]
        pins = {str(int(key)): value for key, value in mappings}
        if len(pins) != len(mappings):
            raise ValueError("duplicate pin")
        project.bindings["parts"][str(args.part)] = {
            "uuid": identity(candidates[0]),
            "pads": pins,
        }
        Project(project.board, project.bindings)
        write_json(args.bindings, project.bindings)
        return 0
    if args.op == "rule":
        endpoints = [
            tuple(map(int, s.split("."))) for s in [args.positive, args.negative]
        ]
        if (
            len(set(endpoints)) != 2
            or any(p not in project.pads for p in endpoints)
            or min(args.id, args.max_length_nm, args.max_skew_nm) < 0
        ):
            raise ValueError("invalid pair rule or unbound terminal")
        project.bindings["rules"][str(args.id)] = {
            "endpoints": endpoints,
            "max_length_nm": args.max_length_nm,
            "max_skew_nm": args.max_skew_nm,
        }
        write_json(args.bindings, project.bindings)
        return 0
    if args.op == "edit":
        run_edits(project, json.loads(args.actions.read_text()), args.pin_table)
        save(project.board, args.board)
    elif args.op == "probe":
        endpoint = (args.part, args.pin)
        print(
            json.dumps(
                {
                    "pads": project.highlight(endpoint),
                    "net": project.pads[endpoint][0].GetNetname(),
                    "reference": project.parts[args.part].GetReference(),
                }
            )
        )
    elif args.op == "locate":
        print(json.dumps(project.reverse[args.uuid]))
    else:
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "drc.json"
            subprocess.run(
                [
                    "kicad-cli",
                    "pcb",
                    "drc",
                    "--format",
                    "json",
                    "--severity-error",
                    "-o",
                    report,
                    args.board,
                ],
                check=True,
                stdout=subprocess.DEVNULL,
            )
            result = json.loads(report.read_text())
        pairs = [
            measure(project, key, args.pair_binary)
            for key in project.bindings.get("rules", {})
        ]
        native = copper(
            project.board, args.copper_binary, project.bindings["copper_limits_nm"]
        )
        print(
            json.dumps(
                {
                    "native_copper": native,
                    "drc": result["violations"],
                    "unconnected": result["unconnected_items"],
                    "pair_margins_nm": pairs,
                }
            )
        )
        return int(
            bool(
                native
                or result["violations"]
                or result["unconnected_items"]
                or any(min(p[-2:]) < 0 for p in pairs)
            )
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
