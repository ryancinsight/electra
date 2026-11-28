"""Check Atlas ownership of direct dependencies and bound Aequitas' inherited closure."""
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
WORKSPACE = {"electra-network", "electra-circuit", "electra-ac", "electra-schematic", "electra-board", "electra-board-field"}
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


def check_direct_adapter(metadata, strict):
    member, = metadata["workspace_members"]
    packages = {p["id"]: p for p in metadata["packages"]}
    adapter = packages[member]
    providers = {"leto", "leto-ops"}
    if adapter["name"] != "electra-leto-direct" or {d["name"] for d in adapter["dependencies"]} != providers | {"electra-circuit", "electra-ac"}:
        raise SystemExit("Unexpected direct provider in the Leto qualification workspace")
    for package in packages.values():
        name = package["name"]
        if name in providers and package["repository"].removesuffix(".git") != "https://github.com/ryancinsight/leto":
            raise SystemExit(f"Unexpected Leto provider repository for {name}")
        if package["id"] == member:
            continue
        if name in WORKSPACE:
            if Path(package["manifest_path"]).resolve() != ROOT / "crates" / name / "Cargo.toml":
                raise SystemExit(f"Unexpected local circuit dependency {name}")
        elif package["source"] != REGISTRY:
            raise SystemExit(f"Unexpected dependency source for {name}")
    inherited = [p for p in packages.values() if not (p["repository"] or "").startswith("https://github.com/ryancinsight/")]
    if strict and inherited:
        raise SystemExit(f"{len(inherited)} non-Atlas transitive package versions in the Leto qualification workspace")
    print(f"Leto qualification workspace: {len(packages)} locked packages, "
          f"{len(inherited)} inherited third-party versions, 0 external direct dependencies")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--strict-transitive", action="store_true",
                        help="require zero third-party packages anywhere in the graph")
    parser.add_argument("--leto-direct", action="store_true", help="audit the opt-in Leto qualification workspace")
    args = parser.parse_args()
    workspace = ROOT / "integrations" / "leto-direct" if args.leto_direct else ROOT
    metadata = json.loads(subprocess.check_output([
        "cargo", "metadata", "--format-version", "1", "--locked", "--offline", "--all-features"
    ], cwd=workspace, text=True))
    (check_direct_adapter if args.leto_direct else check)(metadata, args.strict_transitive)
