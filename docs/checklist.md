# Electra release checklist

Use the checkboxes for evidence state. The [backlog](../backlog.md) owns feature scope and dependencies. Each release inherits the qualified earlier cases. Numerical thresholds below are proposed fixture-specific limits for double precision unless explicitly marked as observed. Each comparison records units, sign convention, fixture revision and expected result. Archive the baseline before acceptance.

<a id="v01"></a>
## 0.1 · DC network foundation

- [x] Execute the divider, unbalanced bridge, parallel branches and reversed source tests. The 9 V divider gives 6 V at the tap, supplies 3 mA and balances power to 1e−15 W. The bridge gives 6/13 V and 5/13 V at the free nodes.
- [x] Solve the two-load rail and recover 4.97 V and 4.95 V within 1e−10 V. Check every free node using absolute 1e−12 A plus relative 1e−10 times its sum of absolute incident currents and prescribed injection. Return an error on floating islands or nonconvergence.
- [x] Check both fixed-end orientations, free–free and fixed–fixed stamps, signed pressure/flow and separately anchored components. Replay the candidate patches through the original CFD matrix and board IR-drop tests, including bit stability. See the [integration evidence](../integrations/README.md).
- [x] Verify formatting, Clippy, docs, workspace tests, the CLI entry point and the locked Atlas-only dependency graph locally.
- [ ] Establish a shared source/package revision, register the member in Atlas, merge each consumer switch with a clean resolved lockfile, and retain the package/install and host CI receipts. Promote 0.1 once these are complete.

<a id="v02"></a>
## 0.2 · General DC and netlists

- [ ] Check floating and controlled voltage/current sources, a supernode, zero-volt current sensing, source reversal, conflicting constraints and a floating island. For well-conditioned fixtures, require 1e−9 relative plus 1e−10 V or 1e−12 A absolute.
- [ ] Compare a DC transfer sweep against an analytical gain and verify one compiled topology is reused. Test hierarchy, name collisions, unit suffixes and bad cards with exact file/line diagnostics.
- [ ] Demonstrate an Atlas-backed, scaled indefinite solve. Compare to analytical cases and track residual, pivot failures, allocations, peak memory and runtime for 100, 1,000 and 10,000 elements. Record all provider and transitive packages.

<a id="v03"></a>
## 0.3 · AC, impedance and noise

- [ ] At the RC pole recover gain 1/√2 and −45° within 0.1% magnitude and 0.1°. Validate complex values at one decade above and below the pole, with phase assessed away from zeros.
- [ ] Recover series RLC resonance and Q within 0.5%, compare a coupled inductor with the ideal transformer limit, and recover resistor one-sided open-circuit noise PSD of 4kTR in V²/Hz within 0.1%.
- [ ] Compare ESR/ESL PDN impedance against the driver over a fixed log grid; document temperature, port convention, treatment of lossless resonance and unsupported model cards.

<a id="v04"></a>
## 0.4 · Transients and events

- [ ] A zero-state RC step reaches 1−e⁻¹ of its final voltage at one time constant within 0.2%. Repeat for RL current. Check underdamped RLC period and logarithmic decrement within 0.5%.
- [ ] Refine the maximum timestep twice with fixed error controls. Record observed order against the chosen method on smooth intervals; bound source-event timing by the event tolerance and compare state before and after rejected steps.
- [ ] Bound integrated KCL and stored/dissipated energy error to 0.2% of supplied charge/energy on the specified nonzero-energy fixture. Reopen a checkpoint and compare the waveform sample by sample.

<a id="v05"></a>
## 0.5 · Active circuits and model coverage

- [ ] Compare diode I–V, BJT operating point, MOSFET transfer and a biased amplifier’s AC gain against redistributable model decks with pinned parameters, temperature and independent reference output. Require maximum 1% relative error above a documented current floor.
- [ ] Simulate a rectifier and a switching converter through startup. Check mean output and ripple within 2% of pinned reference over a specified steady-state window. Distinguish timestep exhaustion, singularity and Newton failure in diagnostics.
- [ ] Qualify a logic-to-analog-to-logic transition with known hysteresis, propagation delay and simultaneous events. Repeat Monte Carlo with the same seed and compare a linear tolerance sweep to its analytical sensitivity.

<a id="v06"></a>
## 0.6 · Schematic to simulation

- [ ] Create and edit a hierarchical schematic in Metis, wire and label nets, assign pin-aware models, cross-probe and undo. Reopening yields identical canonical connectivity, model bindings and part identities.
- [ ] Read/write a pinned KiCad schematic and symbol library. Verify untouched supported fields survive, and unknown fields are preserved or a lossy save is blocked with a diagnostic. Verify import using independently generated fixtures.
- [ ] On a project containing an unconnected required pin and an output conflict, ERC identifies each. Fix both, simulate through the GUI and CLI, and compare traces at matching sample points.

<a id="v07"></a>
## 0.7 · Board and routing

- [ ] Place footprints, route two layers and a differential pair on a four-layer design, add vias, keepouts and a copper zone. Verify that a schematic ECO updates the intended pads and reference designators.
- [ ] DRC catches an injected short, open, clearance, drill/annular-ring and differential-pair mismatch. Removing each fault clears its diagnostic. Round-trip the board and footprint library and compare connectivity, stack-up, coordinates, drill sizes and zone boundaries.
- [ ] Run existing kwavers-driver board consumers through delegating APIs. Preserve routed-pair and UUID contracts, and inventory every extracted module and remaining consumer.

<a id="v08"></a>
## 0.8 · Interconnect and field coupling

- [ ] Recover matched/open/short line reflection coefficients of 0/+1/−1 within 0.01 and the predicted flight time within 1%. Check reciprocity and passivity of a known passive two-port over the declared band.
- [ ] Compare extracted per-unit-length C/L on a defined cross section to an analytical reference within 2%; refine twice and require <1% change on the last refinement. Record mesh, conductor and dielectric assumptions.
- [ ] Compare temperature-dependent rail drop and dissipated power to an independent circuit/thermal balance within 1%. Validate a 1 MHz nominal piezoelectric equivalent circuit with declared motional R/L/C and clamped capacitance; compare impedance extrema and drive power at the electrical port. Measured-device claims require measured parameters and an independent sweep.

<a id="v09"></a>
## 0.9 · Fabrication and workflow

- [ ] Independently read back copper layers, plated/non-plated holes and outline. Compare pad nets and drill counts exactly and feature positions within one declared export grid unit. Check BOM designators and pick-and-place position, units, rotation and board side.
- [ ] On each supported desktop OS, open an earlier project, edit a component value, re-simulate, update layout, run DRC, generate the manufacturing package and reopen it. Preserve a recoverable project after an interrupted save.
- [ ] Record runtime, peak memory, allocations and return status for the pinned 10k-element linear circuit, nonlinear suite and four-layer board. Establish machine-specific budgets from release-mode runs on named hardware before RC.

<a id="v10"></a>
## 1.0 · Stable release

- [ ] At beta publish the supported component/model levels, analyses, SPICE syntax and KiCad format versions; list unsupported fields and diagnostics. Freeze file migration rules and versioned API contracts. Explicitly specify the validated bandwidth/material regime for board field extraction.
- [ ] At RC run three clean-install workflows: discrete amplifier, switching supply and a piezoelectric driver/load board. Archive project inputs, model provenance, reference traces, simulated node currents and power, DRC and manufacturing readback. Reproduce each on macOS, Windows and Linux.
- [ ] Qualify the exact RC against accepted numerical and performance budgets, validate both released Atlas consumers, verify artifacts against the reviewed source tree and dependency closure, and close correctness and compatibility blockers before tagging 1.0.
