"""Verify the entire locked 0.1 graph is local to the Electra workspace."""
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--format-version", "1", "--locked", "--offline", "--all-features"
], cwd=ROOT, text=True))
allowed = {
    (ROOT / "crates" / name / "Cargo.toml").resolve(): name
    for name in ("electra-network", "electra-circuit")
}
for package in metadata["packages"]:
    path = Path(package["manifest_path"]).resolve()
    if package["source"] is not None or allowed.get(path) != package["name"]:
        raise SystemExit(f"Dependency outside the reviewed Atlas graph: {package['id']}")
if {p["name"] for p in metadata["packages"]} != set(allowed.values()):
    raise SystemExit("Locked graph differs from the two expected Electra packages")
print("2 Electra packages; 0 external packages")
