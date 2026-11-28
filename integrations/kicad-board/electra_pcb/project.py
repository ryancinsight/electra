"""Apply a schematic ECO to occurrence-bound footprints and resolve cross-highlights."""

import json
import subprocess
from pathlib import Path
import pcbnew as k


def capture(binary, project):
    data = subprocess.check_output([binary, project], text=True)
    return {
        (int(part), int(pin)): None if net == "off" else int(net)
        for part, pin, net in (row.split() for row in data.splitlines())
    }


def identity(item):
    return item.m_Uuid.AsString()


class Project:
    """The board owns copper and geometry. The binding document owns only cross-domain identities."""

    def __init__(self, board, bindings):
        self.board = board
        self.bindings = bindings
        self.history = []
        self.selection = []
        by_uuid = {identity(fp): fp for fp in board.GetFootprints()}
        self.parts = {
            int(key): by_uuid[bind["uuid"]] for key, bind in bindings["parts"].items()
        }
        self.pads = {}
        self.reverse = {}
        if len({identity(fp) for fp in self.parts.values()}) != len(self.parts):
            raise ValueError("one footprint cannot bind two occurrences")
        for key, binding in bindings["parts"].items():
            fp = self.parts[int(key)]
            by_number = {}
            for pad in fp.Pads():
                by_number.setdefault(pad.GetNumber(), []).append(pad)
            for pin, number in binding["pads"].items():
                endpoint = (int(key), int(pin))
                group = by_number[number]
                for pad in group:
                    if identity(pad) in self.reverse:
                        raise ValueError("physical pad assigned twice")
                    self.reverse[identity(pad)] = endpoint
                self.pads[endpoint] = group

    @classmethod
    def open(cls, path, bindings):
        return cls(k.LoadBoard(str(path)), json.loads(Path(bindings).read_text()))

    def highlight(self, endpoint):
        group = self.pads[endpoint]
        nets = {pad.GetNetCode() for pad in group}
        if len(nets) != 1:
            raise ValueError("inconsistent nets on one physical pin")
        for old in self.selection:
            old.ClearSelected()
        for pad in group:
            pad.SetSelected()
        self.selection = group
        self.board.ResetNetHighLight()
        self.board.SetHighLightNet(group[0].GetNetCode())
        self.board.HighLightON()
        return [identity(pad) for pad in group]

    def _validate_references(self, replacements):
        names = [
            replacements.get(identity(fp), fp.GetReference())
            for fp in self.board.GetFootprints()
        ]
        if any(not name for name in names) or len(set(names)) != len(names):
            raise ValueError("empty or duplicate ECO reference")

    def apply(self, source, references):
        if any(pin not in source for pin in self.pads):
            raise ValueError("bound terminal missing from capture")
        if references.keys() - self.parts.keys():
            raise ValueError("unknown occurrence")
        final_refs = {identity(self.parts[i]): ref for i, ref in references.items()}
        self._validate_references(final_refs)
        # Validate locked targets before changing any pad or allocating a net.
        edits = []
        for i, ref in references.items():
            fp = self.parts[i]
            if fp.GetReference() != ref:
                if fp.IsLocked():
                    raise ValueError("locked component")
                edits.append((fp, fp.GetReference(), ref))
        pads = []
        for endpoint, group in self.pads.items():
            number = source[endpoint]
            name = "" if number is None else f"N{number}"
            for pad in group:
                if pad.GetNetname() != name:
                    if self.parts[endpoint[0]].IsLocked():
                        raise ValueError("locked pad")
                    pads.append((pad, pad.GetNetCode(), name))
        created = {}
        for _, _, name in pads:
            if name and name not in created and self.board.FindNet(name) is None:
                net = k.NETINFO_ITEM(self.board, name)
                self.board.Add(net)
                created[name] = net
        for fp, _, new in edits:
            fp.SetReference(new)
        for pad, _, name in pads:
            pad.SetNetCode(self.board.FindNet(name).GetNetCode() if name else 0)
        if edits or pads:
            self.history.append((edits, pads, created))

    def undo(self):
        if not self.history:
            raise ValueError("no ECO to undo")
        edits, pads, created = self.history[-1]
        self._validate_references({identity(fp): old for fp, old, _ in edits})
        self.history.pop()
        for pad, old, _ in pads:
            pad.SetNetCode(old)
        for fp, old, _ in edits:
            fp.SetReference(old)
        if created:
            used = {
                pad.GetNetCode()
                for fp in self.board.GetFootprints()
                for pad in fp.Pads()
            }
            used.update(t.GetNetCode() for t in self.board.GetTracks())
            used.update(z.GetNetCode() for z in self.board.Zones())
            for net in created.values():
                if net.GetNetCode() not in used:
                    self.board.Remove(net)
