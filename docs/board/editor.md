# Capture-to-board editing

Electra's board tools operate on KiCad 9 boards and footprint libraries through the native pcbnew bindings. The physical layout lives in the KiCad file. A JSON binding document maps schematic occurrence and pin IDs to footprint UUIDs and physical pad numbers. A pair constraint keeps its numeric ID and two capture endpoints across reference-designator and pad-net edits.

## Run the editor

Build the Rust executables and put the output on the path. Use a Python interpreter with KiCad 9 pcbnew.

```sh
cargo build --locked -p electra-board -p electra-schematic --bins
export PATH="$PWD/target/debug:$PATH"
cd integrations/kicad-board
/usr/bin/python3 -m electra_pcb project.kicad_pcb bindings.json init --width-nm 40000000 --height-nm 28000000 --layers 4
/usr/bin/python3 -m electra_pcb project.kicad_pcb bindings.json edit placement.json
/usr/bin/python3 -m electra_pcb project.kicad_pcb bindings.json bind 1 J1 1=1 2=2
/usr/bin/python3 -m electra_pcb project.kicad_pcb bindings.json edit eco.json
/usr/bin/python3 -m electra_pcb project.kicad_pcb bindings.json rule 17 1.1 1.2 23000000 500000
/usr/bin/python3 -m electra_pcb project.kicad_pcb bindings.json probe 1 1
/usr/bin/python3 -m electra_pcb project.kicad_pcb bindings.json check
```

A placement file contains actions such as the following. Supply an installed footprint library path for your system.

```json
[{"op":"place","library":"/usr/share/kicad/footprints/Resistor_SMD.pretty","name":"R_0603_1608Metric","reference":"J1","position":[6000000,8000000]}]
```

An ECO file can contain `[{"op":"eco","schematic":"project.elcp","references":{"1":"R1"}}]`. The command reads the actual ELCP pin table, reconnects bound pads and updates the requested references. Physical net names use `N` followed by the stable ELCP net ID. A physical pad can occur several times on one footprint. Each occurrence of the same pad number receives the same connection. An `undo-eco` action reverses the last ECO in that command session. Locked targets and missing pins fail before the board file is replaced.

Other edit actions include `track` (net, layer, start, end, width), `via` (net, position, diameter, drill), `zone` (net, layer, points) and `keepout` (layer, points). Distances use integer nanometres. The `locate` command maps a board pad UUID back to its capture occurrence and pin. The `net-class` command accepts a class name, width, clearance and one or more schematic net IDs and updates the matching KiCad project file.

## Qualification and limits

The [four-layer replay](../../integrations/kicad-board/verify.py) places two copies of a footprint and a third locked part, applies and reverses an ECO, routes both pair members across F.Cu and B.Cu through four vias, and creates an internal copper zone and a keepout. It reopens the board, verifies pad, track and via positions and identities, checks drill diameters and zone boundaries, and compares IPC-2581 stack layers before and after a second save. The footprint library is independently read and exported with KiCad.

Rule 17 allows 23 mm per member and 0.5 mm absolute skew. Two 20 mm conductors have +3 mm length and +0.5 mm skew margin. A connected 4 mm detour produces −1 mm and −3.5 mm respectively. Undoing the detour restores both original margins. Length is planar track centreline distance. The pair checker rejects arcs and missing members and excludes via barrel length.

The native contact checker sweeps straight copper segments, circular pads and round vias in layer order using a disjoint-set forest for connectivity. Its storage scales with objects, independent of routing-grid pitch. Clearance equality is legal and electrical contact between different nets is a short. KiCad DRC handles the full padstack, shapes, pours, local rules and board outline. The native subset defers open checks on nets with unsupported pads, arcs or filled zones. The combined check returns failure for findings from either engine. The fixture injects and repairs short, open, clearance, drill and ring faults on the same board.
