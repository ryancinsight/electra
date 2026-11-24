# Electra

Electra is the Atlas project for electronic circuit simulation and PCB design. The 0.1.0 source runs a DC operating point for resistor networks, ideal current sources and ideal voltage sources tied to ground. The v1 target is a native schematic-to-simulation-to-fabrication workflow with KiCad file interchange.

```sh
cargo run --locked --offline --bin electra -- op examples/divider.cir
cargo run --locked --offline --bin electra -- op examples/rail.cir
```

The divider yields 6 V at the tap and 3 mA through each resistor. The rail example distributes 100 mA and 200 mA to separate taps of a 5 V supply. Their expected voltages are 4.97 V and 4.95 V.

## Input and results

Use one part per line: `Rname a b ohms`, `Iname a b amperes` or `Vname a b volts`. Values are finite SI numbers such as `1000` or `1e3`. A resistor must have positive, representable conductance. Current flows from a to b. A voltage source enforces Va − Vb, with one terminal at node `0`. Each ideal voltage constraint must own a distinct non-ground node. Node and device names are ASCII-case-insensitive. Full-line comments begin with `*`. The optional control cards are `.op` and a final `.end`. Unsupported elements, directives and engineering suffixes produce line-numbered diagnostics.

The CLI reports node voltages, signed device currents, iteration count and maximum free-node KCL imbalance. Each resistively connected component needs a voltage reference. The Rust API exposes absolute/relative current tolerances and the sweep budget. Gauss–Seidel can converge slowly on long rails and badly conditioned networks. A budget failure returns an error.

## Packages and ownership

| Package | Responsibility |
| --- | --- |
| `electra-network` | Generic two-terminal stamps with fixed-end elimination, ordered relaxation sweeps, anchored linear-network solving and conservation diagnostics. The stamp and sweep monomorphize for the caller’s scalar and storage iterator. |
| `electra-circuit` | Electrical netlist parsing, SI conventions, independent source constraints, branch currents, DC analysis and the `electra` binary. |

Both packages build offline using the Rust standard library and the Electra workspace. The current locked dependency graph has two packages. New capabilities must use Atlas-owned providers. Dependency ownership and extraction points are recorded in the [architecture note](docs/architecture.md).

## Development

```sh
cargo fmt --all -- --check
cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
cargo test --locked --offline --workspace
cargo doc --locked --offline --workspace --no-deps
python3 scripts/check-dependencies.py
```

The [backlog](backlog.md) defines the releases and dependencies. The [release checklist](docs/checklist.md) tracks acceptance evidence, including what is still open in 0.1. The [consumer integration patches](integrations/README.md) adapt the first two shared kernels at their original call sites.
