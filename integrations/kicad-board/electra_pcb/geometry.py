"""Place and route real KiCad objects in integer nanometres."""

import pcbnew as k


def place(board, library, name, ref, position, *, locked=False):
    if not ref or board.FindFootprintByReference(ref) is not None:
        raise ValueError("empty or duplicate footprint reference")
    fp = k.FootprintLoad(str(library), name)
    if fp is None:
        raise ValueError(f"missing footprint {name}")
    fp.SetFPID(k.LIB_ID(library.stem, name))
    fp.SetReference(ref)
    fp.SetPosition(k.VECTOR2I(*position))
    fp.SetLocked(locked)
    board.Add(fp)
    return fp


def segment(board, net, layer, a, b, width):
    if width <= 0 or a == b:
        raise ValueError("track needs positive width and distinct endpoints")
    track = k.PCB_TRACK(board)
    track.SetStart(k.VECTOR2I(*a))
    track.SetEnd(k.VECTOR2I(*b))
    track.SetWidth(width)
    track.SetLayer(layer)
    track.SetNet(net)
    board.Add(track)
    return track


def through_via(board, net, pos, diameter, drill):
    if not 0 < drill < diameter:
        raise ValueError("via drill must lie inside its copper diameter")
    via = k.PCB_VIA(board)
    via.SetPosition(k.VECTOR2I(*pos))
    via.SetWidth(k.PADSTACK.ALL_LAYERS, diameter)
    via.SetDrill(drill)
    via.SetViaType(k.VIATYPE_THROUGH)
    via.SetLayerPair(k.F_Cu, k.B_Cu)
    via.SetNet(net)
    board.Add(via)
    return via


def polygon(board, layer, points, *, net=None):
    if len(set(points)) < 3:
        raise ValueError("zone needs three distinct corners")
    zone = k.ZONE(board)
    zone.SetLayer(layer)
    if net is None:
        zone.SetIsRuleArea(True)
        zone.SetDoNotAllowTracks(True)
        zone.SetDoNotAllowVias(True)
        zone.SetDoNotAllowCopperPour(True)
    else:
        zone.SetNet(net)
        zone.SetLocalClearance(200_000)
    outline = zone.Outline()
    outline.NewOutline()
    for x, y in points:
        outline.Append(x, y)
    board.Add(zone)
    return zone
