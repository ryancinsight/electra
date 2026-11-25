# Electra

Electra is the Atlas project for electronic circuit simulation and PCB design. The headless solver supports linear DC resistor networks with independent sources and all four linear controlled-source families. The 0.2 roadmap increments add DC transfer sweeps, floating voltage sources, zero-volt current probes, typed hierarchical subcircuits and full modified nodal analysis for feedback circuits to the 0.1.0 workspace. The v1 target is a native schematic-to-simulation-to-fabrication workflow with KiCad file interchange.

```sh
cargo run --locked --offline --bin electra -- op examples/divider.cir
cargo run --locked --offline --bin electra -- op examples/rail.cir
cargo run --locked --offline --bin electra -- dc examples/controlled-sources.cir
cargo run --locked --offline --bin electra -- dc examples/transfer.cir
cargo run --locked --offline --bin electra -- op examples/floating-source.cir
cargo run --locked --offline --bin electra -- dc examples/hierarchical-divider.cir
cargo run --locked --offline --bin electra -- dc examples/hierarchical-divider.cir --solver mna
```

The divider yields 6 V at the tap and 3 mA through each resistor. The rail example distributes 100 mA and 200 mA to separate taps of a 5 V supply. Their expected voltages are 4.97 V and 4.95 V.

## Input and results

Each part occupies one line. Use `Rname a b ohms`, `Iname a b amperes` or `Vname a b volts`. A resistor needs a positive, representable conductance. Current flows from a to b. A voltage source enforces Va − Vb between its terminals. Insert a zero-volt source in series to measure signed current. For independent-source decks, every node needs a path to reference node `0` through resistors or voltage sources. Cycles of independent ideal voltage sources yield a line-numbered diagnostic for incompatible imposed voltages or underdetermined branch currents. Names are ASCII-case-insensitive. Full-line comments begin with `*`. A final `.end` is optional.

Use E and G cards to sense a voltage difference, or F and H cards to sense the signed current of a named voltage source. The CLI selects MNA automatically for these circuits. In the example, a +2 mA probe current drives a −6 mA current mirror and a +4 V transresistance output. [Card syntax, feedback examples and qualification](docs/controlled-sources.md)

Primitive numeric literals and sweep values share one parser. It accepts decimal and scientific notation and the case-insensitive scale factors `T G MEG K M U N P F`. For example, `1M` is 0.001 and `1MEG` is 1,000,000. The element or swept source supplies the physical unit. Unknown suffixes and trailing unit text produce a source-line error. The scale factors follow table 2.1 of the [ngspice manual](https://ngspice.sourceforge.io/docs/ngspice-46-manual.pdf).

Use `.subckt` and named Aequitas-typed parameters to instantiate a circuit repeatedly with private nodes. Load model files with `.include` and probe a branch by its instance path. The two-channel example produces 6 V and 3 V from one 9 V rail. See the [syntax, example and qualification results](docs/hierarchical-netlists.md).

Use `electra op FILE` for nominal bias and `electra dc FILE` with one `.dc SOURCE START STOP STEP` directive for a transfer sweep. In the example, `.dc Vin 0 3 500m` produces seven samples and the divider output rises from 0 to 2 V. The nominal 9 V supply still gives 6 V in `op` mode. The signed step can descend. A reachable endpoint is included, and a nondivisible interval ends at the last grid point inside the range.

The Rust API accepts a typed voltage or current range through `Circuit::sweep_source`. Use `Circuit::dc_sweep` for a deck-defined range. Each call to `next_point` returns the excitation and borrows the voltages, device currents and residual data for that point. Copy any traces you want to retain before advancing. See the [verification and performance measurements](docs/dc-transfer.md).

The CLI reports node voltages, signed device currents, iteration count and maximum checked original-node KCL imbalance. For independent-source decks the default reduced solver accepts `--atol-amps`, `--rtol` and `--max-sweeps` after the filename. They default to 1e−12 A, 1e−10 and 100,000 sweeps. The [floating-source qualification](docs/floating-sources.md) relates these controls to observed errors in individual probes. Gauss–Seidel can converge slowly on long rails and badly conditioned networks. A budget failure returns an error.

Select `--solver mna` for an operating point or transfer sweep with Athena Core. It solves node voltages and ideal-source currents together and checks current and voltage residuals separately. Use `--atol-volts` and `--max-iterations` for its source-voltage tolerance and iteration budget. The Rust entry points are `operating_point_mna`, `dc_sweep_mna` and `sweep_source_mna`. [Equations, validation and measured cost](docs/modified-nodal-analysis.md)

For a direct solution, the opt-in Leto adapter reuses one pivoted factor across a DC sweep. It accepts up to 256 non-reference unknowns. [Usage, reference accuracy and memory cost](docs/pivoted-direct.md)

## Packages and ownership

| Package | Responsibility |
| --- | --- |
| `electra-network` | Generic two-terminal stamps with fixed-end elimination, ordered relaxation sweeps, anchored linear-network solving and conservation diagnostics. The stamp and sweep monomorphize for the caller’s scalar and storage iterator. |
| `electra-circuit` | Electrical netlist parsing, SI conventions, independent source constraints, branch currents, DC analysis and the `electra` binary. |

Both crates use Aequitas for dimensional arithmetic. For example, a node voltage multiplied by a branch current produces a value of type `Power`, and a voltage-valued current tolerance fails compilation. The current toolchain floor is Rust 1.95. With dependencies cached, both packages build offline. The locked graph has no third-party direct dependencies and nine inherited third-party package versions through Aequitas/Eunomia. See the [architecture and dependency policy](docs/architecture.md).

The visual workbench is planned. Metis owns UI and interaction; Iris owns visualization. Electra supplies typed electrical data and selection identities to both. The [design evidence](docs/design-evidence.md) connects competitor features and current research to release fixtures.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
cargo test --locked --offline --workspace
cargo doc --locked --offline --workspace --no-deps
python3 scripts/check-dependencies.py
```

The [backlog](backlog.md) defines the releases and dependencies. The [release checklist](docs/checklist.md) tracks acceptance evidence, including what is still open in 0.1. The [consumer integration patches](integrations/README.md) adapt the first two shared kernels at their original call sites.
