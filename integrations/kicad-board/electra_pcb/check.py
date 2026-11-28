"""Pair length checks against stable capture endpoints and routed copper."""

import subprocess
import tempfile
import pcbnew as k


def _run_records(args, records):
    # Spool findings because a checker can emit before it finishes reading. A pipe
    # in both directions could deadlock on a large defective design.
    with tempfile.TemporaryFile(mode="w+t") as output, tempfile.TemporaryFile(
        mode="w+t"
    ) as errors:
        with subprocess.Popen(
            args, stdin=subprocess.PIPE, stdout=output, stderr=errors, text=True
        ) as child:
            with child.stdin as pipe:
                for record in records:
                    pipe.write(record)
                    pipe.write("\n")
            if child.wait():
                errors.seek(0)
                raise ValueError(errors.read().strip() or "board rule check failed")
        output.seek(0)
        yield from output


def _pair_segments(board, pos, neg):
    for t in board.GetTracks():
        if t.GetNetCode() not in (pos, neg) or isinstance(t, k.PCB_VIA):
            continue
        if isinstance(t, k.PCB_ARC):
            raise ValueError("pair length requires straight track segments")
        a, b = t.GetStart(), t.GetEnd()
        yield f"{t.GetNetCode()} {a.x} {a.y} {b.x} {b.y}"


def measure(project, rule_id, binary):
    rule = project.bindings["rules"][str(rule_id)]
    pos, neg = [
        project.pads[tuple(endpoint)][0].GetNetCode() for endpoint in rule["endpoints"]
    ]
    args = [
        str(x)
        for x in (binary, rule_id, pos, neg, rule["max_length_nm"], rule["max_skew_nm"])
    ]
    (row,) = _run_records(args, _pair_segments(project.board, pos, neg))
    key, positive, negative, length, skew = row.split()
    return int(key), *map(float, (positive, negative, length, skew))


def _conductors(board, objects):
    """Stream the supported physical primitives; delegate other shapes to the full KiCad DRC."""
    layers = list(board.GetEnabledLayers().CuStack())
    deferred = set()

    def add(kind, item, a, b, width, span, drill=None):
        objects.append(item.m_Uuid.AsString())
        row = [kind, item.GetNetCode(), a.x, a.y, b.x, b.y, width, *span]
        if drill is not None:
            row.append(drill)
        return " ".join(map(str, row))

    for fp in board.GetFootprints():
        for pad in fp.Pads():
            if pad.GetNetCode() == 0:
                continue
            ls = [i for i, l in enumerate(layers) if pad.IsOnLayer(l)]
            if (
                len(ls) == 0
                or pad.GetShape() != k.PAD_SHAPE_CIRCLE
                or pad.GetSize().x != pad.GetSize().y
                or pad.GetOffset() != k.VECTOR2I(0, 0)
                or len(ls) != ls[-1] - ls[0] + 1
            ):
                deferred.add(pad.GetNetCode())
                continue
            yield add(
                "pad",
                pad,
                pad.GetPosition(),
                pad.GetPosition(),
                pad.GetSize().x,
                [ls[0], ls[-1]],
            )
    for t in board.GetTracks():
        if not t.GetNetCode():
            continue
        if isinstance(t, k.PCB_VIA):
            a, b = sorted([layers.index(t.TopLayer()), layers.index(t.BottomLayer())])
            yield add(
                "via",
                t,
                t.GetPosition(),
                t.GetPosition(),
                t.GetWidth(t.TopLayer()),
                [a, b],
                t.GetDrillValue(),
            )
        elif isinstance(t, k.PCB_ARC):
            deferred.add(t.GetNetCode())
        else:
            l = layers.index(t.GetLayer())
            yield add("track", t, t.GetStart(), t.GetEnd(), t.GetWidth(), [l, l])
    for z in board.Zones():
        if not z.GetIsRuleArea() and any(
            z.HasFilledPolysForLayer(l) and z.GetFilledPolysList(l).OutlineCount()
            for l in layers
        ):
            deferred.add(z.GetNetCode())
    for n in sorted(deferred):
        yield f"defer {n}"


def copper(board, binary, limits):
    objects = []
    findings = []
    for line in _run_records([binary, *map(str, limits)], _conductors(board, objects)):
        kind, *indices = line.split()
        findings.append(
            [
                kind,
                *(
                    [int(indices[0])]
                    if kind == "open"
                    else [objects[int(i)] for i in indices]
                ),
            ]
        )
    return findings
