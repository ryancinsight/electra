"""Report v1 checklist evidence across all milestones."""
from pathlib import Path
import re
import tomllib

root = Path(__file__).resolve().parents[1]
manifest = tomllib.loads((root / "Cargo.toml").read_text())
print(f"Workspace {manifest['workspace']['package']['version']} | Target 1.0")
milestones = {}
for line in (root / "docs/checklist.md").read_text().splitlines():
    heading = re.fullmatch(r"## (\d+\.\d+) · (.+)", line)
    if heading:
        milestone = heading.group(1)
        milestones[milestone] = []
    elif match := re.match(r"^- \[([ xX])\] ", line):
        milestones[milestone].append(match.group(1).lower() == "x")

for version, checks in milestones.items():
    print(f"{version:>3}  {sum(checks)}/{len(checks)} checks")
checks = [check for group in milestones.values() for check in group]
print(f"V1 checklist: {sum(checks)}/{len(checks)} ({sum(checks)/len(checks):.1%})")
