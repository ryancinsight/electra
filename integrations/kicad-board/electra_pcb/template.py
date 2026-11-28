"""Place a translated, routed board block with explicit reference and net bindings."""

import pcbnew as k


def place_layout(board, source, offset, references, nets):
    if tuple(source.GetEnabledLayers().CuStack()) != tuple(
        board.GetEnabledLayers().CuStack()
    ):
        raise ValueError("source and destination must share a copper stack")
    if list(source.Groups()) or any(
        x.GetLayer() != k.Edge_Cuts for x in source.GetDrawings()
    ):
        raise ValueError(
            "layout blocks support footprints, routed copper, zones and keepouts"
        )
    original = list(source.GetFootprints())
    old_refs = [fp.GetReference() for fp in original]
    if len(set(old_refs)) != len(old_refs) or set(references) != set(old_refs):
        raise ValueError("supply one reference mapping for each source footprint")
    targets = set(references.values())
    if (
        "" in targets
        or len(targets) != len(original)
        or any(fp.GetReference() in targets for fp in board.GetFootprints())
    ):
        raise ValueError("empty or occupied destination reference")
    tracks = list(source.GetTracks())
    zones = list(source.Zones())
    if any(list(fp.Zones()) or list(fp.Groups()) for fp in original):
        raise ValueError(
            "nested footprint zones and groups require separate layout bindings"
        )
    source_nets = {pad.GetNetname() for fp in original for pad in fp.Pads()}
    source_nets.update(t.GetNetname() for t in tracks)
    source_nets.update(z.GetNetname() for z in zones if not z.GetIsRuleArea())
    source_nets.discard("")
    if set(nets) != source_nets or any(not n for n in nets.values()):
        raise ValueError("supply a nonempty binding for each source net")
    dest = {}
    for name in set(nets.values()):
        net = board.FindNet(name)
        if net is None:
            net = k.NETINFO_ITEM(board, name)
            board.Add(net)
        dest[name] = net
    move = k.VECTOR2I(*offset)

    def connect(item, name):
        if name:
            item.SetNet(dest[nets[name]])
        else:
            item.SetNetCode(0)

    placed = {}
    for fp in original:
        copy = fp.Duplicate()
        copy.SetReference(references[fp.GetReference()])
        # Read source assignments before attaching the duplicate to a board with different net codes.
        named = [(pad, pad.GetNetname()) for pad in copy.Pads()]
        copy.Move(move)
        board.Add(copy)
        for pad, name in named:
            connect(pad, name)
        placed[fp.GetReference()] = copy
    for item in [*tracks, *zones]:
        name = item.GetNetname()
        copy = item.Duplicate()
        copy.Move(move)
        board.Add(copy)
        if not (isinstance(item, k.ZONE) and item.GetIsRuleArea()):
            connect(copy, name)
    return placed
