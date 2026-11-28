"""Pair length checks against stable capture endpoints and routed copper."""

import subprocess
import pcbnew as k


def measure(project, rule_id, binary):
    rule = project.bindings["rules"][str(rule_id)]
    pos, neg = [
        project.pads[tuple(endpoint)][0].GetNetCode() for endpoint in rule["endpoints"]
    ]
    rows = []
    for t in project.board.GetTracks():
        if t.GetNetCode() not in (pos, neg) or isinstance(t, k.PCB_VIA):
            continue
        if isinstance(t, k.PCB_ARC):
            raise ValueError("pair length requires straight track segments")
        a, b = t.GetStart(), t.GetEnd()
        rows.append(f"{t.GetNetCode()} {a.x} {a.y} {b.x} {b.y}\n")
    args = [
        str(x)
        for x in (binary, rule_id, pos, neg, rule["max_length_nm"], rule["max_skew_nm"])
    ]
    data = subprocess.run(
        args, input="".join(rows), capture_output=True, text=True, check=True
    )
    key, positive, negative, length, skew = data.stdout.split()
    return int(key), *map(float, (positive, negative, length, skew))


def copper(board, binary, limits):
    """Stream the supported physical primitives; delegate other shapes to the full KiCad DRC."""
    layers = list(board.GetEnabledLayers().CuStack())
    objects = []
    rows = []
    deferred = set()

    def add(kind, item, a, b, width, span, drill=None):
        objects.append(item.m_Uuid.AsString())
        row = [kind, item.GetNetCode(), a.x, a.y, b.x, b.y, width, *span]
        if drill is not None:
            row.append(drill)
        rows.append(" ".join(map(str, row)))

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
            add(
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
            add(
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
            add("track", t, t.GetStart(), t.GetEnd(), t.GetWidth(), [l, l])
    for z in board.Zones():
        if not z.GetIsRuleArea() and any(
            z.HasFilledPolysForLayer(l) and z.GetFilledPolysList(l).OutlineCount()
            for l in layers
        ):
            deferred.add(z.GetNetCode())
    rows.extend(f"defer {n}" for n in sorted(deferred))
    data = subprocess.run(
        [binary, *map(str, limits)],
        input="\n".join(rows),
        text=True,
        capture_output=True,
        check=True,
    )
    findings = []
    for line in data.stdout.splitlines():
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
