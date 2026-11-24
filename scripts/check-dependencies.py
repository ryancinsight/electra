"""Check Atlas ownership of direct dependencies and bound Aequitas' inherited closure."""
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
WORKSPACE = {"electra-network", "electra-circuit"}
PROVIDERS = {"athena-core": "athena", "aequitas": "aequitas", "eunomia": "eunomia", "eunomia-derive": "eunomia"}
# Reviewed package versions inherited via Aequitas/Eunomia, including build-time macros.
# Removing one needs no exception. Adding or upgrading one requires a new closure review.
INHERITED = {
    ("typenum", "1.20.1"), ("bytemuck", "1.25.2"), ("bytemuck_derive", "1.12.1"),
    ("libm", "0.2.16"), ("proc-macro2", "1.0.107"), ("quote", "1.0.47"),
    ("syn", "2.0.119"), ("syn", "3.0.10"), ("unicode-ident", "1.0.27"),
}


def check(metadata, strict):
    packages = {p["id"]: p for p in metadata["packages"]}
    members = set(metadata["workspace_members"])
    if {packages[mid]["name"] for mid in members} != WORKSPACE:
        raise SystemExit("Electra workspace membership changed")
    inherited = set()
    for pid, package in packages.items():
        name, version = package["name"], package["version"]
        if pid in members:
            expected = ROOT / "crates" / name / "Cargo.toml"
            if Path(package["manifest_path"]).resolve() != expected or package["source"] is not None:
                raise SystemExit(f"Unexpected workspace source for {name}")
            # Include optional, build, dev, renamed and platform-specific declarations.
            for dep in package["dependencies"]:
                if dep["name"] not in WORKSPACE | PROVIDERS.keys():
                    raise SystemExit(f"External direct dependency: {name} -> {dep['name']}")
            continue
        if package["source"] != REGISTRY:
            raise SystemExit(f"Unreviewed provider source: {pid}")
        if name in PROVIDERS:
            if package["repository"] != f"https://github.com/ryancinsight/{PROVIDERS[name]}":
                raise SystemExit(f"Unrecognized Atlas repository: {pid}")
        elif (name, version) in INHERITED:
            inherited.add((name, version))
        else:
            raise SystemExit(f"Unreviewed transitive package: {pid}")
    if strict and inherited:
        raise SystemExit("Non-Atlas transitive packages: " + ", ".join(
            f"{name}@{version}" for name, version in sorted(inherited)))
    print(f"{len(members)} Electra packages, {len(packages)-len(members)-len(inherited)} Atlas provider packages, "
          f"{len(inherited)} inherited third-party package versions; 0 external direct dependencies")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--strict-transitive", action="store_true",
                        help="require zero third-party packages anywhere in the graph")
    args = parser.parse_args()
    metadata = json.loads(subprocess.check_output([
        "cargo", "metadata", "--format-version", "1", "--locked", "--offline", "--all-features"
    ], cwd=ROOT, text=True))
    check(metadata, args.strict_transitive)
