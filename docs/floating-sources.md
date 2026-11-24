# Independent voltage sources and current probes

A voltage card constrains the difference between two terminals. The current reported for that source is positive from the first terminal to the second. In the [example deck](../examples/floating-source.cir), a 3 V source and a series zero-volt probe connect a 1 kΩ and a 2 kΩ return to ground. The source terminals settle at +1 V and −2 V. The source and probe each read −1 mA. Reversing only the probe terminals reverses its current readout.

## Equations and diagnostics

For a tree of voltage sources, write the potential at each node as the sum of an unknown root potential and a signed offset along the source tree. Summing Kirchhoff’s current law (KCL) over the tree cancels its internal source currents. An external resistor between nodes i and j contributes its ordinary conductance between the roots and a known current G × (offsetᵢ − offsetⱼ). A resistor wholly within the tree still contributes a reported current at its physical terminals. After solving the root potentials, walking the tree from leaves to root recovers each source current from the original circuit’s KCL. [Supernode derivation](https://www-inst.eecs.berkeley.edu/~ee40/su06/lectures/lecture03.pdf)

The reduced network first checks its own conservation. The circuit then checks every recovered source voltage and evaluates KCL in original coordinates. Choosing each tree-edge current closes its child equation. An unbalanced non-reference root can request another relaxation sweep even after the reduced network passes. The circuit computes relative current scales only when the absolute check is insufficient. Source-voltage readback must agree within 1e−12 V plus 1e−9 times the applied magnitude. A nonzero source drop that rounds to zero in the absolute node voltages produces an arithmetic error.

| Input | Result |
| --- | --- |
| An independent source with two non-ground terminals and a resistive or voltage-source path to ground | Unique terminal voltages and signed source current for a well-posed passive network. |
| A connected island referenced only by a current source | A floating-component error carrying an original node index. |
| A closed source loop with inconsistent imposed voltages | A line-numbered incompatible-loop diagnostic naming the source. |
| A consistent closed source loop | A line-numbered redundant-loop diagnostic. Its independent source currents admit infinitely many assignments. |
| A non-finite recovered current, overflowing offset or unresolved source drop | An arithmetic failure. On a sweep-point failure the error includes the excitation and the stream ends. |

The parser selects the source forest in deck order. A closing source in parallel with a tree edge compares imposed voltages exactly. Longer loops accumulate signed voltages along their shared tree path with compensated summation. The closure allowance is eight times binary64 machine epsilon times the smallest nonzero source magnitude on that loop. Contradictory loops take precedence over redundant loops when both are found. Compilation and current recovery use iterative traversals. A regression test solves a 20,000-source series chain with a 1 A load. [Test fixtures](../crates/electra-circuit/tests/voltage_sources.rs)

## Independent accuracy check

The verifier builds and solves conventional modified nodal equations using Python standard-library rational arithmetic. Each of 32 fixed seeds has three ideal-source trees, shunts, inter-node resistors and independent current injections. The verifier runs each circuit in its original and reversed card order and compares every CLI probe, including each voltage-source current. The 0.2 gate for a probe is 1e−9 times its reference magnitude plus 1e−10 V or 1e−12 A.

| CLI settings | Deck solves | Largest voltage error | Largest current error | Probes outside the 0.2 gate |
| --- | ---: | ---: | ---: | ---: |
| Defaults, 1e−12 A absolute and 1e−10 relative KCL | 64 | 2.575e−9 V | 1.704e−10 A | 12 / 2,368 |
| Qualification, 1e−14 A absolute and 1e−13 relative KCL | 64 | 4.796e−12 V | 5.469e−13 A | 0 / 2,368 |

Both settings use the same 100,000-sweep budget. The default settings govern nodal imbalance. Tightening them controls individual probe errors on these fixtures. The default backend handles independent sources through supernode reduction. The [full MNA qualification](modified-nodal-analysis.md) covers the opt-in Athena solver. Controlled-source cases have a separate release gate.

```sh
cargo test --workspace --offline --locked
cargo build --release --offline --locked --bin electra --example source_profile
python3 scripts/verify-dc-mna.py
target/release/electra dc examples/floating-source.cir --atol-amps 1e-14 --rtol 1e-13
```

## Sweep cost

Measured November 24, 2026 on macOS arm64 with Rust 1.95.0 in release mode. The fixture sweeps a common rail from 0 to 1 V at 101 sample points. Each parallel cell contains a 1 kΩ and a 2 kΩ resistor. The floating-source case inserts an independent 1 V source between them. The fixture checks the summed rail current against the closed-form result. Times include preparation, solving and named-probe projection. Parsing and text formatting are excluded. Each time is the median of nine executions after a warm-up, with the order of modes rotated.

| Cells | Grounded at f6ee465 | Grounded now | With a floating source per cell | Peak grounded / floating analysis heap |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 0.111 ms | 0.112 ms | 0.219 ms | 20,286 / 29,069 B |
| 1,000 | 1.201 ms | 1.202 ms | 2.728 ms | 201,186 / 289,169 B |
| 10,000 | 12.158 ms | 12.236 ms | 33.677 ms | 2,010,186 / 2,890,169 B |

A separate system-allocator counter measured additional peak live requested bytes above the parsed fixture. Grounded preparation took 15 allocations at all three sizes, matching the preceding revision. Floating-source preparation took 18. Both paths made zero allocations while advancing the 101 points. The fixture and retained circuit topology precede the measurement. Process RSS and allocator bookkeeping are excluded.

```sh
target/release/examples/source_profile 10000 grounded
target/release/examples/source_profile 10000 floating
```
