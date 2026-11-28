# Held-out placement and routing study

Workspace 0.6.2 compares the original kwavers-driver annealer to an experimental placement policy inspired by the pad alignment, anchor and legalization stages in [Modern Automatic PCB Placement With Complex Constraints](https://ieeexplore.ieee.org/abstract/document/11195800). The experimental policy uses 80 anneal proposals, three net-barycentre sweeps with fourfold weight on locked anchors, and a bounded 7×7 legalization search. The baseline uses 400 anneal proposals. Both receive the identical original placement, fixed connectors, pin offsets, copper access, seeds 71 and 117, a 60-second process budget, and a driver PathFinder router pinned to `34fa3f47` with 0.5 mm pitch, uniform cost and three negotiation iterations.

The two holdouts are the ECC83 amplifier and Xilinx probe from the [Debian KiCad 9.0.2 demo package](https://deb.debian.org/debian/pool/main/k/kicad/kicad-demos_9.0.2+dfsg-1_all.deb). The importer uses physical pad centres, enclosing rectangles, actual pad nets and copper-layer access. Existing routed copper is discarded. The rectangular outline, signal-only class model and footprint bounding boxes simplify the original designs. This is an independent, bounded experiment with the Electra policy. It does not reproduce the paper's industrial benchmarks.

[All eight runs and input SHA-256 digests](placement-results.csv) include connected versus requested nets, overused grid nodes, driver LVS open/short counts, via count, planar track length, KiCad error and unconnected-item counts, wall seconds, and process peak RSS in KiB. KiCad runs separately after routing. It checks the driver-emitted board including physical pad geometry. The Linux memory and time columns cover the placement-and-routing subprocess, including board emission. Wall time is polled to 5 ms. No run timed out.

The experimental policy routed fewer nets on both designs at both seeds. On the Xilinx board it reduced length from 1,024.5 to 753.5 mm at seed 71, while connected nets fell from 36 to 33 of 42. None of the eight layouts passed independent DRC. The experiment stays opt-in.

Build the study and run the corpus as follows, using a Python interpreter with the KiCad 9 pcbnew bindings.

```sh
cargo build --locked --release --manifest-path integrations/place-study/Cargo.toml
/usr/bin/python3 integrations/place-study/replay.py \
  integrations/place-study/target/release/electra-place-study \
  "/usr/share/kicad/demos/ecc83/ecc83-pp.kicad_pcb" \
  "/usr/share/kicad/demos/sonde xilinx/sonde xilinx.kicad_pcb"
```
