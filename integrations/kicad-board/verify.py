"""Four-layer placement, schematic ECO, signed pair rules, independent KiCad DRC and round-trip."""

import argparse
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import pcbnew as k
from electra_pcb import geometry
from electra_pcb.check import measure, copper
from electra_pcb.project import Project, capture, identity

MM = 1_000_000


def mm(x, y):
    return round(x * MM), round(y * MM)


def save(board, path):
    assert k.SaveBoard(str(path), board)


def schematic(path, rows):
    # Independent ELCP v1 producer. Electra's production reader supplies all ECO pin IDs.
    with path.open("wb") as out:
        out.write(b"ELCP\x01\0\0\0" + struct.pack("<Q", len(rows)))
        for instance, pin, net in rows:
            out.write(struct.pack("<QIB", instance, pin, 0 if net is None else 5))
            if net is not None:
                out.write(struct.pack("<Q", net))


def library(parent):
    path = parent / "Electra.pretty"
    path.mkdir()
    fp = k.FOOTPRINT(None)
    fp.SetFPID(k.LIB_ID("Electra", "Dual"))
    fp.SetReference("J**")
    fp.SetValue("Dual")
    for i, y in [(1, 0), (2, 2)]:
        pad = k.PAD(fp)
        pad.SetNumber(str(i))
        pad.SetShape(k.PAD_SHAPE_CIRCLE)
        pad.SetAttribute(k.PAD_ATTRIB_SMD)
        pad.SetSize(k.VECTOR2I(*mm(1, 1)))
        pad.SetPosition(k.VECTOR2I(*mm(0, y)))
        layers = k.LSET()
        layers.AddLayer(k.F_Cu)
        pad.SetLayerSet(layers)
        fp.Add(pad)
    k.PCB_IO_KICAD_SEXPR().FootprintSave(str(path), fp)
    return path


def board_outline(board):
    edge = k.PCB_SHAPE(board)
    edge.SetShape(k.SHAPE_T_RECT)
    edge.SetStart(k.VECTOR2I(*mm(0, 0)))
    edge.SetEnd(k.VECTOR2I(*mm(40, 28)))
    edge.SetLayer(k.Edge_Cuts)
    edge.SetWidth(50000)
    board.Add(edge)


def snapshot(board):
    stack = tuple(board.GetLayerName(x) for x in board.GetEnabledLayers().CuStack())
    footprints = sorted(
        (
            identity(fp),
            fp.GetReference(),
            tuple(fp.GetPosition()),
            fp.IsLocked(),
            sorted(
                (
                    identity(p),
                    p.GetNumber(),
                    p.GetNetname(),
                    tuple(p.GetPosition()),
                    tuple(p.GetSize()),
                )
                for p in fp.Pads()
            ),
        )
        for fp in board.GetFootprints()
    )
    copper = sorted(
        (
            identity(t),
            t.GetNetname(),
            t.GetLayerName(),
            tuple(t.GetStart()),
            tuple(t.GetEnd()),
            (t.GetWidth(k.F_Cu) if isinstance(t, k.PCB_VIA) else t.GetWidth()),
            t.GetDrillValue() if isinstance(t, k.PCB_VIA) else None,
        )
        for t in board.GetTracks()
    )
    zones = sorted(
        (
            identity(z),
            z.GetIsRuleArea(),
            z.GetNetname(),
            z.GetLayerName(),
            tuple(
                tuple(z.Outline().COutline(0).CPoint(i))
                for i in range(z.Outline().COutline(0).PointCount())
            ),
        )
        for z in board.Zones()
    )
    return stack, footprints, copper, zones


def drc(board, path, write=True):
    if write:
        save(board, path)
    report = path.with_suffix(".json")
    subprocess.run(
        [
            "kicad-cli",
            "pcb",
            "drc",
            "--format",
            "json",
            "--severity-error",
            "--all-track-errors",
            "-o",
            report,
            path,
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    result = json.loads(report.read_text())
    return {
        x["type"]
        for group in ("violations", "unconnected_items")
        for x in result[group]
    }


def exercise(directory, reader, checker):
    path = directory / "fixture.kicad_pcb"
    lib = library(directory)
    board = k.BOARD()
    board.SetCopperLayerCount(4)
    board_outline(board)
    save(board, path)
    # seed a distinct copper/dielectric stack independent of the editor writer
    source = path.read_text()
    stack = (
        "(stackup "
        + " ".join(
            f'(layer "{name}" (type "{kind}") (thickness {thick})'
            + (
                ' (material "FR4") (epsilon_r 4.2) (loss_tangent 0.02)'
                if kind != "copper"
                else ""
            )
            + ")"
            for name, kind, thick in [
                ("F.Cu", "copper", 0.035),
                ("dielectric 1", "prepreg", 0.18),
                ("In1.Cu", "copper", 0.018),
                ("dielectric 2", "core", 1.1),
                ("In2.Cu", "copper", 0.018),
                ("dielectric 3", "prepreg", 0.18),
                ("B.Cu", "copper", 0.035),
            ]
        )
        + ")"
    )
    assert "(stackup" not in source
    path.write_text(source.replace("(setup", "(setup " + stack, 1))
    board = k.LoadBoard(str(path))
    bindings = {
        "copper_limits_nm": [200000, 200000, 100000],
        "parts": {},
        "rules": {
            "17": {
                "endpoints": [[1, 1], [1, 2]],
                "max_length_nm": 23 * MM,
                "max_skew_nm": MM // 2,
            }
        },
    }
    for i, (ref, x, y) in enumerate([("J1", 6, 8), ("J2", 26, 8), ("J3", 32, 18)], 1):
        fp = geometry.place(board, lib, "Dual", ref, mm(x, y), locked=(i == 3))
        bindings["parts"][str(i)] = {"uuid": identity(fp), "pads": {"1": "1", "2": "2"}}
    bindpath = directory / "capture-board.json"
    bindpath.write_text(json.dumps(bindings))
    editor = Project(board, bindings)
    rows = [(1, 1, 1), (1, 2, 2), (2, 1, 2), (2, 2, 1), (3, 1, None), (3, 2, None)]
    sch = directory / "project.elcp"
    schematic(sch, rows)
    editor.apply(capture(reader, sch), {})
    original_locked = snapshot(board)[1]
    endpoint = (2, 1)
    pad_id = editor.highlight(endpoint)[0]
    assert editor.reverse[pad_id] == endpoint
    rows[2:4] = [(2, 1, 1), (2, 2, 2)]
    schematic(sch, rows)
    source = capture(reader, sch)
    editor.apply(source, {2: "J7"})
    assert editor.pads[endpoint][0].GetNetname() == "N1"
    editor.undo()
    assert (
        editor.parts[2].GetReference() == "J2"
        and editor.pads[endpoint][0].GetNetname() == "N2"
    )
    assert editor.highlight(endpoint) == [pad_id]
    editor.apply(source, {2: "J7"})
    assert next(x for x in original_locked if x[1] == "J3") == next(
        x for x in snapshot(board)[1] if x[1] == "J3"
    )
    assert editor.pads[(2, 2)][0].GetNetname() == "N2"
    p, n = board.FindNet("N1"), board.FindNet("N2")

    def track(net, layer, a, b):
        return geometry.segment(board, net, layer, mm(*a), mm(*b), 200_000)

    paths = []
    for net, y in [(p, 8), (n, 10)]:
        segs = [
            track(net, k.F_Cu, (6, y), (11, y)),
            track(net, k.B_Cu, (11, y), (21, y)),
            track(net, k.F_Cu, (21, y), (26, y)),
        ]
        vias = [
            geometry.through_via(board, net, mm(x, y), 900_000, 400_000)
            for x in (11, 21)
        ]
        paths.append((segs, vias))
    pour = geometry.polygon(
        board, k.In1_Cu, [mm(9, 5), mm(13, 5), mm(13, 9), mm(9, 9)], net=p
    )
    geometry.polygon(board, k.In2_Cu, [mm(12, 17), mm(17, 17), mm(17, 22), mm(12, 22)])
    assert k.ZONE_FILLER(board).Fill(board.Zones())
    assert pour.GetFilledPolysList(k.In1_Cu).OutlineCount() > 0
    assert measure(editor, 17, checker) == (17, 20 * MM, 20 * MM, 3 * MM, MM // 2)

    def native():
        return {
            r[0]
            for r in copper(
                board,
                Path(checker).with_name("electra-copper"),
                [200_000, 200_000, 100_000],
            )
        }

    assert not native()
    base = drc(board, path)
    print("baseline", sorted(base))
    assert not base, base

    def fault(kind, inject, repair):
        inject()
        assert {
            "unconnected_items": "open",
            "clearance": "clearance",
            "shorting_items": "short",
            "annular_width": "annular_ring",
            "drill_out_of_range": "drill",
        }[kind] in native()
        errors = drc(board, path)
        assert kind in errors, (kind, errors)
        repair()
        after = drc(board, path)
        assert after == base, after
        print("detected and repaired", kind)

    route = paths[1][0][0]
    old = k.VECTOR2I(*tuple(route.GetEnd()))
    fault(
        "unconnected_items",
        lambda: route.SetEnd(k.VECTOR2I(*mm(9, 10))),
        lambda: route.SetEnd(old),
    )
    # Bring a foreign-net spur alongside or across the positive trace.
    for kind, y in [("clearance", 8.25), ("shorting_items", 8)]:
        x = 6 if kind == "shorting_items" else 8
        spur = track(n, k.F_Cu, (x, 10), (x, y))
        assert {
            "unconnected_items": "open",
            "clearance": "clearance",
            "shorting_items": "short",
            "annular_width": "annular_ring",
            "drill_out_of_range": "drill",
        }[kind] in native()
        errors = drc(board, path)
        assert kind in errors, (kind, errors)
        board.Remove(spur)
        after = drc(board, path)
        assert after == base, after
        print("detected and repaired", kind)
    via = paths[0][1][0]
    fault("annular_width", lambda: via.SetDrill(890_000), lambda: via.SetDrill(400_000))
    fault(
        "drill_out_of_range",
        lambda: via.SetDrill(50_000),
        lambda: via.SetDrill(400_000),
    )
    # Replace a 10 mm member segment with a 14 mm connected excursion.
    middle = paths[1][0][1]
    board.Remove(middle)
    detour = [
        track(n, k.B_Cu, a, b)
        for a, b in [((11, 10), (11, 12)), ((11, 12), (21, 12)), ((21, 12), (21, 10))]
    ]
    assert measure(editor, 17, checker) == (17, 20 * MM, 24 * MM, -MM, -3.5 * MM)
    for item in detour:
        board.Remove(item)
    board.Add(middle)
    assert measure(editor, 17, checker) == (17, 20 * MM, 20 * MM, 3 * MM, MM // 2)
    after = drc(board, path)
    assert after == base, after
    from xml.etree import ElementTree as ET

    def exported_stack():
        output = directory / "board.xml"
        subprocess.run(
            ["kicad-cli", "pcb", "export", "ipc2581", "-o", output, path],
            check=True,
            stdout=subprocess.DEVNULL,
        )
        xml = ET.parse(output)
        return [
            tuple(sorted(item.attrib.items()))
            for item in xml.findall(".//{*}StackupLayer")
        ]

    stack = exported_stack()
    assert len(stack) >= 7, stack
    expected = snapshot(board)
    reopened = Project.open(path, bindpath)
    assert not native()
    assert expected == snapshot(reopened.board)
    save(reopened.board, path)
    assert exported_stack() == stack
    assert (
        "(thickness 1.1)" in path.read_text() and '(material "FR4")' in path.read_text()
    )
    assert (
        reopened.highlight(endpoint) == [pad_id]
        and reopened.reverse[pad_id] == endpoint
    )
    assert reopened.board.IsHighLightNetON() and all(
        p.IsSelected() and p.GetNetname() == "N1" for p in reopened.pads[endpoint]
    )
    assert measure(reopened, 17, checker) == (17, 20 * MM, 20 * MM, 3 * MM, MM // 2)
    fp = k.FootprintLoad(str(lib), "Dual")
    assert sorted(
        (p.GetNumber(), tuple(p.GetPosition()), tuple(p.GetSize())) for p in fp.Pads()
    ) == [("1", mm(0, 0), mm(1, 1)), ("2", mm(0, 2), mm(1, 1))]
    subprocess.run(
        ["kicad-cli", "fp", "export", "svg", "-o", directory / "fp-svg", lib],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    import sys

    def cli(*args):
        return subprocess.check_output(
            [sys.executable, "-m", "electra_pcb", path, bindpath, *args],
            cwd=Path(__file__).resolve().parent,
            text=True,
        )

    assert json.loads(cli("probe", "2", "1"))["pads"] == [pad_id]
    assert json.loads(cli("locate", pad_id)) == [2, 1]
    checked = json.loads(
        cli(
            "check",
            "--pair-binary",
            checker,
            "--copper-binary",
            str(Path(checker).with_name("electra-copper")),
        )
    )
    assert not any(checked[key] for key in ("native_copper", "drc", "unconnected"))
    cli("net-class", "Pair", "200000", "180000", "1", "2")
    settings = json.loads(path.with_suffix(".kicad_pro").read_text())["net_settings"]
    assert (
        next(c for c in settings["classes"] if c["name"] == "Pair")["track_width"]
        == 0.2
        and len(settings["netclass_patterns"]) == 2
    )
    cli("net-class", "Pair", "200000", "4000000", "1", "2")
    assert "clearance" in drc(reopened.board, path, write=False)
    cli("net-class", "Pair", "200000", "180000", "1", "2")
    assert not drc(reopened.board, path, write=False)
    from layout_replay import verify_layout

    verify_layout(directory, path, reader, checker, lib, source)
    print(
        "four-layer board, footprint library, cross-highlighting and shared rule 17 passed"
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("pin_table")
    parser.add_argument("pair_check")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as temp:
        exercise(Path(temp), args.pin_table, args.pair_check)
