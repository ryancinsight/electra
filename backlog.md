# Electra backlog

The release sequence below defines the v1 product scope. Each item has a dependency, deliverable and acceptance fixture. Implementation evidence lives in the [release checklist](docs/checklist.md). Versions are targets. The 0.1 source is available for local verification.

<a id="electra-01"></a>
## ELECTRA-01 — 0.1 Resistive DC and shared network math [minor] — todo

- priority: architecture
- needs: none
- Deliverable: Ship the headless resistor-network API and CLI. Extract CFDrs’ boundary-eliminating stamp and kwavers-driver’s ordered relaxation sweep. Preserve public driver behavior and CFD unit boundaries. Register the repo and verify first-party dependency provenance.
- Exit: Packaged divider and PCB rail, a hydraulic network, floating-island rejection, exact consumer revision replay.
- [Acceptance checklist](docs/checklist.md#v01)

<a id="electra-02"></a>
## ELECTRA-02 — 0.2 Modified nodal analysis and a real netlist model [minor] — todo

- priority: correctness
- needs: 0.1
- Deliverable: Introduce node and branch unknowns, independent floating voltage sources, the four controlled-source families, DC sweeps and reusable symbolic topology. Add hierarchical subcircuits, model/parameter scoping, common numeric suffixes and source-line diagnostics. Integrate Aequitas, Leto and Athena only after qualifying the actual provider closure and indefinite solve path.
- Exit: Reversing a floating source reverses its measured polarity. Rank deficiency and incompatible ideal sources have distinct diagnostics. Voltage and current gains match analytical controlled-source cases.
- [Acceptance checklist](docs/checklist.md#v02)

<a id="electra-03"></a>
## ELECTRA-03 — 0.3 Linear frequency response and noise [minor] — todo

- priority: feature
- needs: 0.2
- Deliverable: Add capacitors, inductors, mutual inductance, ideal transformers and small-signal AC stamps. Sweep complex frequency, calculate input impedance and gain/phase, and propagate independent thermal-noise sources. Move the driver’s PDN impedance models and document ESR/ESL conventions.
- Exit: RC cutoff, RLC resonance/Q, transformer ratio, thermal noise density and the legacy capacitor bank agree with independent references.
- [Acceptance checklist](docs/checklist.md#v03)

<a id="electra-04"></a>
## ELECTRA-04 — 0.4 Transient simulation and switching waveforms [minor] — todo

- priority: correctness
- needs: 0.2, 0.3
- Deliverable: Add charge/flux state, initial conditions, pulse/PWL/sine sources, switch events, adaptive implicit integration, local error estimates, timestep rejection and restart. Qualify Horae’s DAE/event coverage or extend it there. Add time-domain RLC and line models with explicit units.
- Exit: RC/RL steps and RLC ringdown converge under timestep refinement. Charge and energy errors are bounded across discontinuities.
- [Acceptance checklist](docs/checklist.md#v04)

<a id="electra-05"></a>
## ELECTRA-05 — 0.5 Semiconductors, mixed-signal behavior and tolerances [minor] — todo

- priority: feature
- needs: 0.3, 0.4
- Deliverable: Add documented diode, BJT and MOSFET model levels, temperature dependence, bounded behavioral sources, event-driven digital primitives and analog/digital thresholds. Implement Newton damping/continuation, linearization at the operating point, sensitivity, corners and seeded Monte Carlo. Specify supported SPICE model cards and unsupported-card diagnostics.
- Exit: Rectifier, transistor bias stage, switched converter, logic threshold crossing and seeded tolerance sweep meet independent DC/AC/transient reference envelopes.
- [Acceptance checklist](docs/checklist.md#v05)

<a id="electra-06"></a>
## ELECTRA-06 — 0.6 Native schematic capture and KiCad interchange [minor] — todo

- priority: feature
- needs: 0.2, 0.5
- Deliverable: Introduce the project/schematic/symbol model with stable IDs, pins, hierarchy, buses, labels, junctions and ERC. Use Metis for native editing and waveform plots. Bind symbols to models and physical pin numbers. Extract the existing schematic writer and add a versioned reader with preservation or explicit rejection of unmodeled fields.
- Exit: Open, edit, undo, save and reopen a two-sheet project. Netlist generation and CLI/editor simulation agree. Cross-probing locates the same net and device after round-trip.
- [Acceptance checklist](docs/checklist.md#v06)

<a id="electra-07"></a>
## ELECTRA-07 — 0.7 PCB editing, routing and design rules [minor] — todo

- priority: feature
- needs: 0.6
- Deliverable: Extract the board, geometry, placement and routing slices from kwavers-driver. Edit footprints, layers, tracks, vias, copper zones and net classes. Implement schematic-to-PCB updates, undo, rule provenance and native DRC. Support routed differential pairs and board/footprint interchange.
- Exit: Route a four-layer board, inject open/short/clearance/annular-ring/differential-pair faults, and verify exact pad-to-net connectivity after KiCad round-trip.
- [Acceptance checklist](docs/checklist.md#v07)

<a id="electra-08"></a>
## ELECTRA-08 — 0.8 Board-aware electrical and multiphysics modeling [minor] — todo

- priority: feature
- needs: 0.3, 0.4, 0.7
- Deliverable: Model routed RLGC, transmission lines, termination and reflections, multiport parameters, PDN and DC IR drop. Couple copper loss to temperature and resistance. Qualify a bounded electromagnetic field solve for trace cross-section capacitance/inductance, then assemble a board-derived circuit. Add a Butterworth–Van Dyke transducer load as a kwavers adapter fixture.
- Exit: Matched line, open/short reflection, passive reciprocal two-port, mesh-converged cross-section, thermal rail and loaded piezoelectric port each have independent evidence.
- [Acceptance checklist](docs/checklist.md#v08)

<a id="electra-09"></a>
## ELECTRA-09 — 0.9 Fabrication output and native end-to-end qualification [minor] — todo

- priority: verification
- needs: 0.5–0.8
- Deliverable: Generate Gerber, Excellon, layer map, pick-and-place and BOM; add inspection of rendered output and an independent fabrication readback. Qualify model/footprint library resolution, project recovery, migrations and reproducible export. Exercise the native workflow on macOS, Windows and Linux.
- Exit: A fresh install edits, simulates, routes, checks, exports and reopens the same project. Measured connectivity, holes and outlines survive manufacturing readback.
- [Acceptance checklist](docs/checklist.md#v09)

<a id="electra-10"></a>
## ELECTRA-10 — 1.0 Stable electrical design and simulation workflow [major] — todo

- priority: verification
- needs: 0.1–0.9
- Deliverable: Freeze the V1 component, analysis and file-format support matrix at beta. RC qualification covers public API and project compatibility, independent analytical/differential data, numerical failure modes, performance, packages and both actual Atlas consumers.
- Exit: Replay all accepted cases from clean installations at the exact RC revision. Gate the final tag on complete recorded evidence and consumer adoption.
- [Acceptance checklist](docs/checklist.md#v10)
