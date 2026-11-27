"""Round-trip installed KiCad 9 reference documents and import both edits with KiCad."""
import argparse
from pathlib import Path
import subprocess
import tempfile
import xml.etree.ElementTree as ET

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--share", type=Path, default=Path("/usr/share/kicad"))
share = parser.parse_args().share
subprocess.run(["cargo", "build", "--locked", "-p", "electra-schematic", "--example", "kicad-property"], cwd=root, check=True)
editor = root / "target/debug/examples/kicad-property"


def edit(source, target, symbol, key, before, after):
    raw = source.read_bytes()
    with target.open("wb") as output:
        subprocess.run([editor, source, symbol, key, after], stdout=output, check=True)
    marker = f'(property "{key}" "{before}"'.encode()
    updated = f'(property "{key}" "{after}"'.encode()
    data = target.read_bytes()
    assert updated in data and data.replace(updated, marker, 1) == raw


with tempfile.TemporaryDirectory() as temporary:
    work = Path(temporary)
    sch = work / "Arduino_Nano.kicad_sch"
    edit(share / "template/Arduino_Nano" / sch.name, sch,
         "00000000-0000-0000-0000-000056d71a21", "Value", "Digital", "Electra digital")
    netlist = work / "export.xml"
    subprocess.run(["kicad-cli", "sch", "export", "netlist", "--format", "kicadxml",
                    "-o", netlist, sch], check=True)
    assert ET.parse(netlist).findtext(".//comp[@ref='J2']/value") == "Electra digital"
    lib = work / "Regulator_Current.kicad_sym"
    edit(share / "symbols" / lib.name, lib, "HV100K5-G", "Description",
         "Hot-Swap Current Limiter Controller, SOT223", "Electra library replay")
    subprocess.run(["kicad-cli", "sym", "export", "svg", "--symbol", "HV100K5-G",
                    "-o", work / "svg", lib], check=True)
    assert list((work / "svg").glob("*.svg"))
    print("KiCad imported both edits; all bytes outside each property value matched.")
