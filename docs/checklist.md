# Electra release checklist

Use the checkboxes for evidence state. The [design evidence](design-evidence.md) gives sources, study limitations and the reference workflow for each added gate. The [backlog](../backlog.md) owns feature scope and dependencies. Each release inherits the qualified earlier cases. Numerical thresholds below are proposed fixture-specific limits for double precision unless explicitly marked as observed. Each comparison records units, sign convention, fixture revision and expected result. Archive the baseline before acceptance.

<a id="v01"></a>
## 0.1 · DC network foundation

- [x] Aequitas carries voltages, currents, resistive impedances, conductances, hydraulic pressures/flows, residuals and tolerances through assembly and solve. Run three compile-fail cases for a voltage/current probe, a wrongly dimensioned tolerance and a hydraulic/electrical mismatch, plus the typed power calculation.
- [ ] Reduce the Aequitas/Eunomia transitive dependency closure in the providers and pass the strict transitive audit. The default audit currently allows nine reviewed inherited package versions.

- [x] Execute the divider, unbalanced bridge, parallel branches and reversed source tests. The 9 V divider gives 6 V at the tap, supplies 3 mA and balances power to 1e−15 W. The bridge gives 6/13 V and 5/13 V at the free nodes.
- [x] Solve the two-load rail and recover 4.97 V and 4.95 V within 1e−10 V. Check every free node using absolute 1e−12 A plus relative 1e−10 times its sum of absolute incident currents and prescribed injection. Return an error on floating islands or nonconvergence.
- [x] Check both fixed-end orientations, free–free and fixed–fixed stamps, signed pressure/flow and separately anchored components. Replay the candidate patches through the original CFD matrix and board IR-drop tests, including bit stability. See the [integration evidence](../integrations/README.md).
- [x] Verify formatting, Clippy, docs, workspace tests, the CLI entry point and the locked direct-dependency and inherited-closure audit locally.
- [ ] Establish a shared source/package revision, register the member in Atlas, merge each consumer switch with a clean resolved lockfile, and retain the package/install and host CI receipts. Promote 0.1 once these are complete.

<a id="v02"></a>
## 0.2 · General DC and netlists

- [x] Reproduce an eSim-style subcircuit with two parameterized instances. Check every voltage and branch current against a flat deck and closed form. A time-valued resistor override identifies its instance, parameter and conflicting units. [Example and results](hierarchical-netlists.md)
- [x] Verify the full MNA ideal-source blocks. Check original-node current imbalance in amperes and individual ideal-source voltage error in volts, including zero-diagonal and disparate-scale cases. [Qualification](modified-nodal-analysis.md)

- [x] Check floating independent voltage sources, source reversal, a zero-volt probe, branching source trees, floating islands, incompatible voltage loops and nonunique source currents. Verify original-node KCL at each accepted point. Complete the 20,000-source chain with iterative traversal.
- [x] Compare all node voltages and individual device currents against exact rational MNA on 32 circuits in two deck orders. At 1e−14 A absolute and 1e−13 relative solver tolerances, all probes meet 1e−9 relative plus 1e−10 V or 1e−12 A absolute. At the default KCL limits, 12 of 2,368 probe comparisons exceed this stricter gate. [Verification and performance](floating-sources.md)
- [x] Qualify E/G/F/H with analytical gains, signed branch currents, negative feedback, scoped controls and singular zero-drive cases. All 18,432 controlled-source probe comparisons passed per tolerance setting across flat and hierarchical decks. [Qualification and allocation measurements](controlled-sources.md)
- [x] Sweep independent current sources and ground-referenced voltage sources in both directions using the CLI and typed API. Match the 2/3 divider gain, signed source currents and a 25-point bridge against its two-equation solution. Verify decimal endpoints, nondivisible ranges, numerical failures and nominal-bias isolation.
- [x] Reuse one compiled topology and bounded workspace per sweep. Validate each point against the original physical conservation criterion. Measure elapsed time and peak live solver heap for 101 points at 100, 1,000 and 10,000 free nodes. See the [fixture and results](dc-transfer.md).
- [x] Parse case-insensitive T/G/MEG/K/M/U/N/P/F scale factors for part and sweep cards. Reject bad units, source types, ranges, trailing fields and duplicate controls with a one-based deck line.
- [x] Test hierarchical name collisions, lexical parameter scope, cross-file diagnostics and qualified sweeps. Verify 4,096-level traversal, empty-subtree pruning and rejection of unrepresentable expanded device counts. Compare all probes on 64 hierarchical decks to exact rational MNA at the qualification tolerances. [Evidence](hierarchical-netlists.md)
- [x] Demonstrate scaled indefinite DC analysis with Athena Core GMRES. Compare every bias and sweep probe with exact rational MNA. Record numerical breakdown and singularity diagnostics, peak memory, allocations, runtime and the locked provider closure at 100, 1,000 and 10,000 physical elements. [Measurements](modified-nodal-analysis.md)
- [x] Qualify Leto direct LU on the indefinite DC fixtures. All 46,848 probes passed at each tolerance. Record eight false singularities in the published sparse path, replay the correction, and measure dense fallback memory. Use an explicitly selected factor through 256 unknowns. [Pivot and memory evidence](pivoted-direct.md)

<a id="v03"></a>
## 0.3 · AC, impedance and noise

- [x] Stamp typed passive R/C/L branches on arbitrary node pairs using shared boundary elimination. Recover both independent bridge equations with complex drive and reversed endpoints. [API, range checks and assembly measurements](passive-ac-stamps.md)
- [x] Parse flat R/C/L decks with grounded phasor sources and export the assembled sparse equations. Verify both exported rows against an independent complex bridge solution. [Deck qualification](passive-ac-stamps.md#flat-ac-netlists)
- [x] Solve reducible cyclic AC networks and resonant leaf constraints. Check all ring voltages against SciPy and limit dense allocation by remaining core size. [Qualification](ac-direct.md#cyclic-components-and-resonant-leaves)
- [x] Solve large connected passive trees without allocating a dense matrix. Verify every voltage against an independent sparse solver and check physical KCL. [Connected tree qualification](ac-direct.md#connected-trees-above-128-free-nodes)
- [x] Partition the frequency-specific reduced system into independent components and reuse the bounded solver for decks above 128 total free nodes. [Qualification](ac-direct.md#independent-channels)
- [x] Solve a parsed passive AC deck with the bounded Leto backend. Verify solved voltages and KCL against the headless ladder and independent bridge solution. [Solver and qualification](ac-direct.md)

- [x] Specify the Iris visual contract for Bode and PSD data. A 1 kHz cursor reads 1,000 Hz from the headless samples. Reject a current trace bound to a voltage axis; distinguish V²/Hz from V/√Hz and absolute dB references from dimensionless gain. [Typed conversion and cursor qualification](rc-frequency-response.md#spectral-readback-contract)

- [x] At the RC pole recover gain 1/√2 and −45° within 0.1% magnitude and 0.1°. Validate complex values one decade above and below the pole, assessing phase away from zeros. Compare gain and port impedance against independent nodal solutions through the CLI. [RC evidence](rc-frequency-response.md)
- [x] Recover series RLC resonance and Q within 0.5% using the reactance zero and the two half-power points. [Series-branch qualification](capacitor-bank.md)
- [x] Recover one-sided open-circuit resistor noise PSD of 4kTR in V²/Hz within 0.1%. Verify nonuniform-temperature RC ladders against independent nodal power sums and read the pole PSD from the original Iris samples. [Thermal-noise evidence](thermal-noise.md)
- [x] Compare coupled windings with the ideal transformer limit for gain and reflected impedance. Verify finite magnetizing current, secondary polarity, open and short terminations and real-power balance. [Independent winding equations and sweep measurements](coupled-transformers.md)
- [x] Compare ESR/ESL PDN impedance against the driver over a fixed log grid and document temperature, port convention, lossless resonance and supported inputs. [606 CLI comparisons and singular cases](capacitor-bank.md)

<a id="v04"></a>
## 0.4 · Transients and events

- [x] Restart the sampled mixed-signal fixture at the first ideal rectifier drive switch and first ADC transition. At 0.1 s clocks and 10⁻⁷ V/A absolute tolerances, both runs matched the exact LC reference on all 13 edge indices and directions (zero missed or duplicated). All 400 sample pairs replayed bit for bit. Peak errors were 72.1 µV and 69.7 µA. Both runs accepted 3,229 substeps and rejected 1,127 trials with zero failed intervals on the valid waveform. Across 11 Linux x86-64 release test processes, median uninterrupted and restarted fixture times were 239 and 209 µs in that order (includes reference/trace collection); median peak RSS for the combined test was 1,496 KiB. Checkpoint size was 64 bytes. The comparator observes clock boundaries and uses an externally held threshold. [Executable fixture](../crates/electra-circuit/tests/sampled_restart.rs)

- [x] A zero-state RC step reaches 1−e⁻¹ of its final voltage at one time constant within 0.2%. Repeat for RL current. The typed backward Euler models pass at 256 equal steps per time constant and show first-order convergence at 64, 128 and 256 steps.
- [x] Check underdamped RLC period and logarithmic decrement within 0.5%. For a 2 Ω, 0.1 H, 1 mF series branch released from 1 V and zero current, successive positive voltage peaks give 0.00494% period error and 0.01395% logarithmic decrement error at 0.25 ms. Halving the step twice from 1 ms reduced period error about fourfold per refinement. [Executable fixture](../crates/electra-circuit/tests/rlc_ringdown.rs)
- [x] Refine the maximum timestep twice with fixed error controls. At 40, 20 and 10 ms limits, with 10 µV and 10 µA tolerances, the normalized endpoint error fell by factors of 3.99 and 3.97 on a driven 1 H, 1 F lossless branch. Phase readback at the prescribed 0.73 s source change was within 30 µs. A forced rejected trial recovered the exact voltage and current of four manually replayed quarter steps. [Adaptive transient regression](../crates/electra-circuit/tests/adaptive_transient.rs)
- [x] Bound integrated KCL and stored/dissipated energy error to 0.2% of supplied charge/energy. The zero-state 2 Ω, 0.1 H, 1 mF series fixture uses 1,000 steps of 0.1 ms and changes drive from 3 V to 1 V at step 400. Final relative charge and energy errors were 1.01 × 10⁻¹⁵ and 1.02 × 10⁻¹³. Reopening the 64-byte checkpoint before the drive change reproduced every subsequent voltage and current sample bit for bit. [Executable fixture](../crates/electra-circuit/tests/rlc_checkpoint.rs)

<a id="v05"></a>
## 0.5 · Active circuits and model coverage

- [x] Qualify independent uniform resistor tolerances on a 9 V unloaded divider with upper resistance 950–1,050 Ω and lower resistance 1,900–2,100 Ω. The four corners give extrema 342/59 V and 378/61 V. Two 16,384-draw runs at seed 2026 match the six-bin [exact integer reference](../scripts/verify-divider-histogram.py) at thresholds 5.8, 5.9, 6.0, 6.1 and 6.2 V. [Rust regression](../crates/electra-circuit/tests/divider_tolerance.rs)
- [ ] Replay an eSim-style digital driver feeding an RC analog load. Compare the first threshold time with the analytical crossing and replay simultaneous events in documented order. Compare candidate adjoint sensitivities to a step-refined central difference and closed-form divider derivatives.

- [ ] Compare diode I–V, BJT operating point, MOSFET transfer and a biased amplifier’s AC gain against redistributable model decks with pinned parameters, temperature and independent reference output. Require maximum 1% relative error above a documented current floor.
- [ ] Simulate a rectifier and a switching converter through startup. Check mean output and ripple within 2% of pinned reference over a specified steady-state window. Distinguish timestep exhaustion, singularity and Newton failure in diagnostics.
- [ ] Qualify a logic-to-analog-to-logic transition with known hysteresis, propagation delay and simultaneous events. Repeat Monte Carlo with the same seed and compare a linear tolerance sweep to its analytical sensitivity.

<a id="v06"></a>
## 0.6 · Schematic to simulation

- [ ] Author and inspect two instances of an EasyEDA Pro-style reusable schematic block. Inject a swapped package pin and missing power pin; assert exact offending IDs. Use Metis pointer/keyboard input to select a hierarchical net and query the corresponding labeled Iris current/voltage traces after save/reopen.
- [ ] Qualify Metis input/undo/selection and Iris axis/cursor extensions in their owning repos. Record provider revisions and a native Electra replay with numerical samples equal to the CLI.

- [ ] Create and edit a hierarchical schematic in Metis, wire and label nets, assign pin-aware models, cross-probe and undo. Reopening yields identical canonical connectivity, model bindings and part identities.
- [ ] Read/write a pinned KiCad schematic and symbol library. Verify untouched supported fields survive, and unknown fields are preserved or a lossy save is blocked with a diagnostic. Verify import using independently generated fixtures.
- [ ] On a project containing an unconnected required pin and an output conflict, ERC identifies each. Fix both, simulate through the GUI and CLI, and compare traces at matching sample points.

<a id="v07"></a>
## 0.7 · Board and routing

- [ ] Exercise an OrCAD X-style shared constraint ID, EasyEDA Pro layout reuse/ECO with an unrelated locked instance, and DipTrace-style pair tuning. A known length/skew violation and repair must produce the expected signed margin. Verify cross-highlighting survives rename, undo and reload.
- [ ] Run the research placer against the driver placer on held-out boards under the same router, seeds and time limit. Publish completion, DRC faults, vias, total length, memory and time, including failed routes.

- [ ] Place footprints, route two layers and a differential pair on a four-layer design, add vias, keepouts and a copper zone. Verify that a schematic ECO updates the intended pads and reference designators.
- [ ] DRC catches an injected short, open, clearance, drill/annular-ring and differential-pair mismatch. Removing each fault clears its diagnostic. Round-trip the board and footprint library and compare connectivity, stack-up, coordinates, drill sizes and zone boundaries.
- [ ] Run existing kwavers-driver board consumers through delegating APIs. Preserve routed-pair and UUID contracts, and inventory every extracted module and remaining consumer.

<a id="v08"></a>
## 0.8 · Interconnect and field coupling

- [ ] Compare two PCB heater layouts at recorded input power, stack-up and ambient conditions. Validate simulated peak and spatial temperature rise against measured locations and uncertainty; record coupon-specific limits.
- [ ] On a pinned board compare fixed-via and joint via/decap candidates under the same component budget against the same target impedance over frequency. Verify the winning trace independently. Detect an injected passivity violation, measure fit change after repair, and check energy into passive terminations.
- [ ] Use Metis to probe an Iris rail-drop and thermal overlay at a known board coordinate. Check value, unit, physical position and selected net against solver samples at two zoom levels.

- [ ] Recover matched/open/short line reflection coefficients of 0/+1/−1 within 0.01 and the predicted flight time within 1%. Check reciprocity and passivity of a known passive two-port over the declared band.
- [ ] Compare extracted per-unit-length C/L on a defined cross section to an analytical reference within 2%; refine twice and require <1% change on the last refinement. Record mesh, conductor and dielectric assumptions.
- [ ] Compare temperature-dependent rail drop and dissipated power to an independent circuit/thermal balance within 1%. Validate a 1 MHz nominal piezoelectric equivalent circuit with declared motional R/L/C and clamped capacitance; compare impedance extrema and drive power at the electrical port. Measured-device claims require measured parameters and an independent sweep.

<a id="v09"></a>
## 0.9 · Fabrication and workflow

- [ ] Reopen a DipTrace-style STEP export in an independent reader. Match connector position, rotation and height to within one declared linear/angular export tick. Inspect board layers, units and orientation through the Metis/Iris 3D preview.

- [ ] Independently read back copper layers, plated/non-plated holes and outline. Compare pad nets and drill counts exactly and feature positions within one declared export grid unit. Check BOM designators and pick-and-place position, units, rotation and board side.
- [ ] On each supported desktop OS, open an earlier project, edit a component value, re-simulate, update layout, run DRC, generate the manufacturing package and reopen it. Preserve a recoverable project after an interrupted save.
- [ ] Record runtime, peak memory, allocations and return status for the pinned 10k-element linear circuit, nonlinear suite and four-layer board. Establish machine-specific budgets from release-mode runs on named hardware before RC.

<a id="v10"></a>
## 1.0 · Stable release

- [ ] Update the dated competitor and research register at each minor and the RC. Record search window, method, source maturity, tested regime and adopt/experiment/defer decision for relevant new work.
- [ ] Audit every physical public API and serialized field for Aequitas dimension and unit, check display/parse round-trips, and trace each graphical surface to Metis and Iris. Re-run provider integration fixtures and list supported competitor workflows and remaining gaps.

- [ ] At beta publish the supported component/model levels, analyses, SPICE syntax and KiCad format versions; list unsupported fields and diagnostics. Freeze file migration rules and versioned API contracts. Explicitly specify the validated bandwidth/material regime for board field extraction.
- [ ] At RC run three clean-install workflows: discrete amplifier, switching supply and a piezoelectric driver/load board. Archive project inputs, model provenance, reference traces, simulated node currents and power, DRC and manufacturing readback. Reproduce each on macOS, Windows and Linux.
- [ ] Qualify the exact RC against accepted numerical and performance budgets, validate both released Atlas consumers, verify artifacts against the reviewed source tree and dependency closure, and close correctness and compatibility blockers before tagging 1.0.
