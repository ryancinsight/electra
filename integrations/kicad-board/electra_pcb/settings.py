"""KiCad 9 project class edits and durable identity-file writes."""

import json
from pathlib import Path
import tempfile
import os


def write_json(path, data):
    path = Path(path)
    with tempfile.NamedTemporaryFile(
        mode="w", delete=False, dir=path.parent, encoding="utf8"
    ) as target:
        name = target.name
        try:
            json.dump(data, target, indent=2, ensure_ascii=False, allow_nan=False)
            target.write("\n")
            target.flush()
            os.fsync(target.fileno())
        except BaseException:
            os.unlink(name)
            raise
    try:
        os.replace(name, path)
    finally:
        Path(name).unlink(missing_ok=True)


def set_class(path, name, nets, width_nm, clearance_nm):
    if (
        not name
        or name == "Default"
        or width_nm <= 0
        or clearance_nm < 0
        or not nets
        or min(nets) < 0
    ):
        raise ValueError("invalid net class")
    path = Path(path)
    data = json.loads(path.read_text()) if path.exists() else {}
    settings = data.setdefault("net_settings", {})
    settings.setdefault("meta", {"version": 4})
    classes = settings.setdefault("classes", [])
    current = next((c for c in classes if c["name"] == name), None)
    if current is None:
        current = {"name": name, "priority": 0}
        classes.append(current)
    current.update(track_width=width_nm / 1_000_000, clearance=clearance_nm / 1_000_000)
    old = settings.setdefault("netclass_patterns", [])
    targets = {f"N{n}" for n in nets}
    # Explicit net names take precedence over inherited wildcard patterns.
    settings["netclass_patterns"] = [
        {"pattern": target, "netclass": name} for target in sorted(targets)
    ] + [row for row in old if row.get("pattern") not in targets]
    write_json(path, data)
