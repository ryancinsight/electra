"""Export the actual pad centres, rotated bounds and net assignments from a KiCad board."""
import sys
import pcbnew as k

def export(path, out):
    b = k.LoadBoard(str(path))
    bounds = b.GetBoardEdgesBoundingBox()
    origin = bounds.GetOrigin()
    layers = list(b.GetEnabledLayers().CuStack())
    print("board", bounds.GetWidth(), bounds.GetHeight(), len(layers), file=out)
    for fp in sorted(b.GetFootprints(), key=lambda f: (f.GetReference(), f.m_Uuid.AsString())):
        pos = fp.GetPosition()
        box = fp.GetBoundingBox(False, False)
        half_w = max(pos.x - box.GetLeft(), box.GetRight() - pos.x)
        half_h = max(pos.y - box.GetTop(), box.GetBottom() - pos.y)
        ref = fp.GetReference()
        print("part", ref, pos.x-origin.x, pos.y-origin.y, 2*half_w, 2*half_h,
              int(fp.IsLocked() or ref.startswith(("J","P"))), file=out)
        for pad in fp.Pads():
            p = pad.GetPosition()
            bounds = pad.GetBoundingBox()
            access = [str(i) for i, layer in enumerate(layers) if pad.IsOnLayer(layer)]
            if access and pad.GetAttribute() != k.PAD_ATTRIB_NPTH:
                print("pad", p.x-pos.x, p.y-pos.y, bounds.GetWidth(), bounds.GetHeight(),
                      pad.GetNetCode(), ",".join(access), file=out)

if __name__ == "__main__":
    export(sys.argv[1], sys.stdout)
