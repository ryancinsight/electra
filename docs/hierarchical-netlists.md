# Hierarchical DC netlists

The [divider deck](../examples/hierarchical-divider.cir) instantiates the same [three-part model](../examples/subcircuits/divider.cir) twice. At 9 V the left channel gives 6 V and the right gives 3 V. Both zero-volt probes read +3 mA from junction to output. The supply branch reads −6 mA. Run the example from the repository root.

```sh
cargo run --locked --offline --bin electra -- op examples/hierarchical-divider.cir
cargo run --locked --offline --bin electra -- dc examples/hierarchical-divider.cir
```

## Model and instance syntax

```spice
.subckt divider supply return sense params: upper=1kOhm lower=2kOhm
Rupper supply junction {upper}
Vsense junction sense 0
Rlower sense return {lower}
.ends divider
```

```spice
.include "subcircuits/divider.cir"
.param trim=2kOhm
Vin rail 0 9
Xleft rail 0 left divider
Xright rail 0 right divider upper={trim} lower=1kOhm
.dc Vin 0 9 3
.end
```

| Construct | Meaning |
| --- | --- |
| `.subckt NAME pins... [params: key=value ...]` and `.ends [NAME]` | Define a reusable model at file scope. Pins bind positionally. Models can call other models and can precede or follow their callers. All model names share a deck-wide namespace. |
| `Xname nodes... MODEL [key=value ...]` | Instantiate the model. Each named override applies only to that instance. An optional `params:` may precede the overrides. |
| `.param key=value ...` | Declare constants local to the containing model or root. Declaration order within that scope is immaterial. A caller passes its parameters explicitly to children. Only parameters in the model header can be overridden. |
| `{key}` | Read a typed parameter in a primitive card or forward it in an instance override. Each reference must have the dimension required by its destination. |
| `.include "relative path"` | Load at file scope, relative to the file containing the directive. Paths without whitespace can omit quotes. Each file can have its own final `.end`. Include and model cycles produce diagnostics. |
| `Xouter/Xinner/Vsense` | Address an instance's branch in CLI and API output, or as an independent source in `.dc`. Internal nodes use the same prefix. Node `0` is shared throughout the circuit. |

Parameter defaults and local declarations require a numeric literal ending in `Ohm`, `V`, `A` or `s`. The first three unit labels accept ASCII case variants. Seconds use lowercase `s`. A scale factor can precede the unit. The model signature supplies the dimension for an unlabelled instance override such as `upper=2k`. Primitive numeric literals and sweep ranges use the existing element/source unit convention. A resistor parameter overridden by `1ms` reports the instance, parameter, expected ohms and supplied seconds. Seconds can be declared and forwarded for dimension checking. The implemented analyses are operating point and DC sweep.

Hierarchical identifiers are ASCII-case-insensitive and reserve `/` for the instance path. The first spelling of a node is retained for display. Each instance owns its internal nodes. An unused pin passed through any number of models contributes no circuit unknown. Use `electra_circuit::parse_file` for file-backed designs and `electra_circuit::parse` for a self-contained in-memory deck. A file diagnostic carries the originating path and one-based line, including when different actual pins collapse two terminals of a primitive.

This subset covers literal parameter defaults, single-parameter references and named overrides. Arithmetic expressions, `.model` cards, continuation lines and nested model declarations remain outside the grammar. The comparison workflow is model reuse and node/branch probing in eSim. The ngspice manual documents the broader parameter syntax. [eSim resources](https://esim.fossee.in/resources) · [ngspice subcircuit parameters](https://nmg.gitlab.io/ngspice-manual/circuitdescription/paramparametricnetlists/subcircuitparameters.html)

## Correctness evidence

The divider test checks all six node voltages and seven branch currents against a hand-flattened deck and closed form at nominal bias and at 0, 3, 6 and 9 V. Separate tests sweep a nested source by its case-insensitive path, exercise independent sibling parameters, retain the signs of both source currents and verify local diagnostics through the CLI. A 4,096-level chain exercises iterative traversal. A binary tree of repeated empty models checks pruning. With a physical leaf, a tree whose part count exceeds the machine index range fails at the root instance before expansion.

The rational MNA check uses 32 seeded networks in two card orders. Each device is placed inside two model instances, with reversed formal pin order at the inner boundary and a forwarded typed parameter. The reference assembles its matrix directly from the original terminal pairs. It checks the complete set of published probe names as well as the voltages and signed currents. Each tolerance setting checks 2,368 probes across 64 hierarchical decks.

| KCL solver tolerances | Maximum voltage error | Maximum current error | Probes outside the fixture gate |
| --- | ---: | ---: | ---: |
| Default, 1e−12 A absolute and 1e−10 relative | 2.575e−9 V | 1.704e−10 A | 12 |
| Qualification, 1e−14 A absolute and 1e−13 relative | 4.796e−12 V | 5.469e−13 A | 0 |

The per-probe gate is 1e−9 relative plus 1e−10 V or 1e−12 A absolute. The flat encodings gave the same maximum errors and counts.

```sh
cargo test --locked --offline -p electra-circuit --test hierarchy
cargo build --release --locked --offline --bin electra --example hierarchy_profile
python3 scripts/verify-dc-mna.py --hierarchical
python3 scripts/verify-dc-mna.py
```

## Parsing and memory

Measured November 24, 2026 on macOS arm64, Rust 1.95.0, release mode. Each parallel divider has two resistors and a private midpoint. Alternating cells use 1 kΩ or 4 kΩ above a 2 kΩ load. The benchmark checks the supply current against the analytical sum. Timings are medians of nine fresh executions after a warm-up, rotating between the three encodings/revisions. Timing starts after input generation and ends when a checked circuit is available. Solver time is measured separately.

| Cells | Flat at 3a1639f | Flat now | Hierarchical now | Peak additional parser heap in the same order, bytes |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 0.066 ms | 0.055 ms | 0.057 ms | 37,148 / 36,653 / 35,896 |
| 1,000 | 0.471 ms | 0.401 ms | 0.345 ms | 441,230 / 435,959 / 334,488 |
| 10,000 | 4.751 ms | 3.955 ms | 2.947 ms | 4,787,900 / 4,719,005 / 3,680,208 |

At 10,000 cells the three runs retained 3,381,366 / 3,381,366 / 2,419,982 bytes in the parsed circuit and made 90,077 / 50,074 / 40,121 allocation or reallocation requests during parsing. A separate system allocator counter sampled additional live requested bytes starting immediately before parse. Input text, process RSS, allocator bookkeeping and solver workspace are excluded. Median operating-point times for the same three cases were 0.324 / 0.333 / 0.335 ms. These figures characterize repeated two-resistor dividers on this machine.

```sh
target/release/examples/hierarchy_profile 10000 flat
target/release/examples/hierarchy_profile 10000 hierarchical
```
