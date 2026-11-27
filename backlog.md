# Electra backlog

The release sequence below defines the v1 product scope. Each item has a dependency, deliverable and acceptance fixture. Implementation evidence lives in the [release checklist](docs/checklist.md). Versions are targets. Each verified development increment between minor milestones advances the workspace patch version. Completing the next minor milestone advances the minor version and resets the patch to zero. The 0.2.0 source recorded general DC, 0.3.0 recorded AC qualification, and the 0.4.0 workspace records transient fixture qualification. Workspace 0.4.1 records the first 0.5 increment, [independent divider tolerance analysis](https://github.com/ryancinsight/electra/pull/22). Workspace 0.4.2 adds ordered RC drive updates and divider adjoint sensitivity. Workspace 0.4.3 adds sampled Schmitt hysteresis with inertial delay and a single pending edge. Workspace 0.4.4 adds the fixed-temperature ideal junction current and differential conductance API. Workspace 0.4.5 qualifies ideal NPN, forward-channel square-law NMOS and a resistively loaded common-source amplifier against ngspice. These are static branch APIs; nonlinear deck solving remains open. Package publication and consumer adoption are tracked by their release gates. The [design evidence](docs/design-evidence.md) records official product sources, recent papers, limitations and acceptance fixtures. Each minor release refreshes this register. All GUI work goes through Metis and all visualization through Iris. Extend reusable capabilities in the owning provider. Physical values use Aequitas across storage, simulation, display and export.

<a id="electra-01"></a>
## ELECTRA-01 — 0.1 Resistive DC and shared network math [minor] — todo

- priority: architecture
- needs: none
- Deliverable: Ship the headless resistor-network API and CLI. Extract CFDrs’ boundary-eliminating stamp and kwavers-driver’s ordered relaxation sweep. Preserve public driver behavior and CFD unit boundaries. Register the repo and verify first-party dependency provenance.
- Exit: Packaged divider and PCB rail, a hydraulic network, floating-island rejection, exact consumer revision replay.
- Carry Aequitas types through both shared kernels, both consumers, diagnostics and tolerances. Keep the upstream third-party closure visible and reduce it in the providers.
- [Acceptance checklist](docs/checklist.md#v01)

<a id="electra-02"></a>
## ELECTRA-02 — 0.2 Modified nodal analysis and a real netlist model [minor] — todo

- priority: correctness
- needs: 0.1
- Deliverable: Introduce node and branch unknowns, independent floating voltage sources, the four controlled-source families, DC sweeps and reusable symbolic topology. Add hierarchical subcircuits, model/parameter scoping, common numeric suffixes and source-line diagnostics. Carry the existing Aequitas types into voltage/current unknowns and model parameters. Qualify the Leto/Athena closure and indefinite solve path.
- Exit: Reversing a floating source reverses its measured polarity. Rank deficiency and incompatible ideal sources have distinct diagnostics. Voltage and current gains match analytical controlled-source cases.
- Implemented the streaming, single-source DC transfer analysis for independent I and grounded V sources, typed ranges, reusable compressed topology, and nine common numeric scale factors. The [transfer qualification](docs/dc-transfer.md) records analytical and allocation results. The second increment adds arbitrarily connected independent voltage sources, zero-volt probes, loop diagnostics, CLI convergence settings and original-node KCL checks. The [floating-source qualification](docs/floating-sources.md) covers signed branch currents and exact-reference accuracy.
- Implemented E, G, F and H controlled sources with dimensioned gains, scoped branch references, feedback rank checks and automatic CLI solver selection. The [controlled-source qualification](docs/controlled-sources.md) records the analytical gains, exact-reference comparisons and 101-point sweep measurements.
- Implemented the opt-in scaled full MNA operating-point and sweep backend with Athena Core. Exact rational, physical residual and 100–10,000-element resource results are in the [MNA qualification](docs/modified-nodal-analysis.md). The [pivoted direct qualification](docs/pivoted-direct.md) closes the 0.2 numerical checklist with a bounded Leto adapter and a provider regression for sparse pivot handoff. Release promotion still depends on the 0.1 packaging and consumer gates.
- Implemented reusable hierarchical DC subcircuits, Aequitas-typed parameter overrides, relative file includes and qualified node/branch probes for the eSim reuse workflow. The [hierarchy qualification](docs/hierarchical-netlists.md) records the dialect, reference comparisons and peak parser memory.
- [Acceptance checklist](docs/checklist.md#v02)

<a id="electra-03"></a>
## ELECTRA-03 — 0.3 Linear frequency response and noise [minor] — todo

- priority: feature
- needs: 0.2
- Qualified lossless coupled windings and ideal transformers under voltage drive, including load reflection, polarity, open and shorted secondaries, and the finite-magnetizing limit. [Transformer qualification](docs/coupled-transformers.md)
- Implemented the [series RLC and parallel capacitor-bank model](docs/capacitor-bank.md) with complex port impedance, resonance and Q checks, and differential tests against the Kwavers driver. Driver adoption remains open.
- Implemented the loaded RC ladder, streaming sweeps and a borrowed Iris Bode view. Typed RMS voltage, dBV, gain in dB and voltage amplitude spectral density complete the headless Iris readback contract. The [passive nodal API](docs/passive-ac-stamps.md) now stamps arbitrary R/C/L connections. Flat AC decks with grounded voltage sources now export reduced nodal equations through the CLI. The [bounded AC direct solver](docs/ac-direct.md) uses pivoted LU through 128 free voltages per component, supports larger trees and reducible cyclic networks via sparse elimination and independent factorization of disconnected residual blocks, and checks physical branch KCL. Large cores with half-bandwidth up to 32 under the initial or farthest-level breadth-first ordering now use complex banded pivoting. General sparse pivoting for wider cores above 128 nodes, floating source support for AC, and native display remain open. [RC qualification](docs/rc-frequency-response.md)
- Implemented independent series and shunt resistor noise for RC ladders and a typed, borrowed Iris voltage PSD trace. [Thermal-noise qualification](docs/thermal-noise.md)
- Deliverable: Add capacitors, inductors, mutual inductance, ideal transformers and small-signal AC stamps. Sweep complex frequency, calculate input impedance and gain/phase, and propagate independent thermal-noise sources. Move the driver’s PDN impedance models and document ESR/ESL conventions.
- Exit: RC cutoff, RLC resonance/Q, transformer ratio, thermal noise density and the legacy capacitor bank agree with independent references.
- Define typed complex impedance, phase, spectral density and reference quantities. Design the Iris Bode/PSD view contract so exported samples and cursor readback use the same units.
- [Acceptance checklist](docs/checklist.md#v03)

<a id="electra-04"></a>
## ELECTRA-04 — 0.4 Transient simulation and switching waveforms [minor] — todo

- priority: correctness
- needs: 0.2, 0.3
- Implemented typed, allocation-free series RC and RL backward Euler steps under piecewise-constant voltage drive. The caller supplies the initial state and step duration. The series RLC model advances capacitor voltage and inductor current together using an allocation-free implicit midpoint step. Fixed-step series RLC checkpoints preserve component values, state, interval and step index for exact replay with the same external drive schedule. Step-doubling control adapts the series RLC midpoint step to absolute voltage and current tolerances and clips it at caller-specified source events. Sample-clock integration reports one-bit ADC edges and resumes from the existing checkpoint under the same external schedule and threshold. The scoped ideal rectifier and ADC fixture is qualified; nonlinear rectifier device models remain under 0.5.
- Deliverable: Add charge/flux state, initial conditions, pulse/PWL/sine sources, switch events, adaptive implicit integration, local error estimates, timestep rejection and restart. Qualify Horae’s DAE/event coverage or extend it there. Add time-domain RLC and line models with explicit units.
- Exit: RC/RL steps and RLC ringdown converge under timestep refinement. Charge and energy errors are bounded across discontinuities.
- Apply the mixed-signal research baseline to timestep rejection, discontinuities and event ordering. Track accuracy alongside convergence failures and runtime.
- [Acceptance checklist](docs/checklist.md#v04)

<a id="electra-05"></a>
## ELECTRA-05 — 0.5 Semiconductors, mixed-signal behavior and tolerances [minor] — todo

- priority: feature
- needs: 0.3, 0.4
- Deliverable: Add documented diode, BJT and MOSFET model levels, temperature dependence, bounded behavioral sources, event-driven digital primitives and analog/digital thresholds. Implement Newton damping/continuation, linearization at the operating point, sensitivity, corners and seeded Monte Carlo. Specify supported SPICE model cards and unsupported-card diagnostics.
- Exit: Rectifier, transistor bias stage, switched converter, logic threshold crossing and seeded tolerance sweep meet independent DC/AC/transient reference envelopes.
- Implemented typed resistance intervals, four-corner output and constant-storage seeded voltage sampling for an unloaded divider. Unloaded divider sensitivity uses the single-node adjoint. General netlist tolerance sweeps and adjoints remain open.
- Use OrCAD X/PSpice worst-case and Monte Carlo workflows and eSim mixed-signal models as comparison targets. Evaluate ADSPICE-style sensitivity against closed-form and finite-difference oracles before proposing an optimized implementation.
- [Acceptance checklist](docs/checklist.md#v05)

<a id="electra-06"></a>
## ELECTRA-06 — 0.6 Native schematic capture and KiCad interchange [minor] — todo

- priority: feature
- needs: 0.2, 0.5
- Deliverable: Introduce the project/schematic/symbol model with stable IDs, pins, hierarchy, buses, labels, junctions and ERC. Use Metis for native editing and interaction, and Iris for waveform plots, axes and cursor readback. Bind symbols to models and physical pin numbers. Extract the existing schematic writer and add a versioned reader with preservation or explicit rejection of unmodeled fields.
- Exit: Open, edit, undo, save and reopen a two-sheet project. Netlist generation and CLI/editor simulation agree. Cross-probing locates the same net and device after round-trip.
- Qualify the eSim model/subcircuit authoring and node/branch probing workflow, EasyEDA Pro reusable channel blocks, and DipTrace hierarchical projects. Use package-pin fault injection from the research register.
- [Acceptance checklist](docs/checklist.md#v06)

<a id="electra-07"></a>
## ELECTRA-07 — 0.7 PCB editing, routing and design rules [minor] — todo

- priority: feature
- needs: 0.6
- Deliverable: Extract the board, geometry, placement and routing slices from kwavers-driver. Edit footprints, layers, tracks, vias, copper zones and net classes. Implement schematic-to-PCB updates, undo, rule provenance and native DRC. Support routed differential pairs and board/footprint interchange.
- Exit: Route a four-layer board, inject open/short/clearance/annular-ring/differential-pair faults, and verify exact pad-to-net connectivity after KiCad round-trip.
- Carry OrCAD X rule identities across capture and layout, replay EasyEDA Pro template and ECO updates, and tune differential pairs against DipTrace-style length and skew budgets. Benchmark placement using a fixed router and held-out designs.
- [Acceptance checklist](docs/checklist.md#v07)

<a id="electra-08"></a>
## ELECTRA-08 — 0.8 Board-aware electrical and multiphysics modeling [minor] — todo

- priority: feature
- needs: 0.3, 0.4, 0.7
- Deliverable: Model routed RLGC, transmission lines, termination and reflections, multiport parameters, PDN and DC IR drop. Couple copper loss to temperature and resistance. Qualify a bounded electromagnetic field solve for trace cross-section capacitance/inductance, then assemble a board-derived circuit. Add a Butterworth–Van Dyke transducer load as a kwavers adapter fixture.
- Exit: Matched line, open/short reflection, passive reciprocal two-port, mesh-converged cross-section, thermal rail and loaded piezoelectric port each have independent evidence.
- Assess via/decap co-design against a fixed-via PDN; qualify passive S-parameter models against independent frequency and transient evidence. Present IR drop, impedance and temperature through typed Iris views within Metis.
- [Acceptance checklist](docs/checklist.md#v08)

<a id="electra-09"></a>
## ELECTRA-09 — 0.9 Fabrication output and native end-to-end qualification [minor] — todo

- priority: verification
- needs: 0.5–0.8
- Deliverable: Generate Gerber, Excellon, layer map, pick-and-place and BOM; add inspection of rendered output and an independent fabrication readback. Qualify model/footprint library resolution, project recovery, migrations and reproducible export. Exercise the native workflow on macOS, Windows and Linux.
- Exit: A fresh install edits, simulates, routes, checks, exports and reopens the same project. Measured connectivity, holes and outlines survive manufacturing readback.
- Add a DipTrace-style 3D/STEP mechanical export fixture with known layer heights, transforms and component orientations. Verify manufacturing previews through Metis/Iris and independent readback.
- [Acceptance checklist](docs/checklist.md#v09)

<a id="electra-10"></a>
## ELECTRA-10 — 1.0 Stable electrical design and simulation workflow [major] — todo

- priority: verification
- needs: 0.1–0.9
- Deliverable: Freeze the V1 component, analysis and file-format support matrix at beta. RC qualification covers public API and project compatibility, independent analytical/differential data, numerical failure modes, performance, packages and both actual Atlas consumers.
- Exit: Replay all accepted cases from clean installations at the exact RC revision. Gate the final tag on complete recorded evidence and consumer adoption.
- Refresh all named commercial/open-source baselines and research sources. Close the dimensional API and presentation-provider audits and publish a capability-by-capability support matrix.
- [Acceptance checklist](docs/checklist.md#v10)
