# DC transfer qualification

The 0.2 increment sweeps one independent current source or ground-referenced voltage source through a passive resistor network. Each point is an independent operating point with the nominal conservation tolerances. The session compiles the connections once and keeps one set of solver and output buffers.

## Analytical checks

| Fixture | Independent expectation | What the test checks |
| --- | --- | --- |
| 1 kΩ / 2 kΩ divider | Output is ⅔ of the input, and the supply current in passive sign convention is −Vin / 3000 Ω. | Seven samples from 0 to 3 V in 0.5 V steps, within 1e−12 V and 1e−15 A. Running nominal bias afterwards returns 6 V. |
| Two-resistor current transfer | A current source from a to b feeds a 2 Ω shunt at a and a 4 Ω shunt at b. With a separate 0.25 A injection, Va = 2 Ω × (0.25 A − I) and Vb = 4 Ω × I. | Increments from +1 A down to −1 A preserve both signs and the bias even when the swept source has a nominal value of 1e25 A. |
| Two-supply resistor bridge | With a 0.125 A bias and supply voltage S, set b = S + 0.125 in SI magnitudes. The free nodes are (5b − 8) / 16 and (2b − 16) / 16 V. These follow by inverting the independent 2×2 nodal equations. | All five node voltages and all nine signed device currents on 25 samples from −3 to +3 V. Includes parallel branches of opposite orientations and a reversed fixed supply. |

The public API and CLI tests also cover forward source references, bad control cards, missing and duplicate directives, nominal-bias isolation, equal endpoints, partial final intervals, rounding at decimal endpoints, and a representable three-point transfer across a span wider than f64::MAX. An unrepresentable increment or failed solve terminates the stream. A compile-fail example rejects a voltage-valued current sweep. [Circuit tests](../crates/electra-circuit/tests/dc_sweep.rs) · [Network tests](../crates/electra-network/tests/conservation.rs)

## Runtime and heap

Measured November 24, 2026 on macOS arm64 with Rust 1.95.0 in release mode. Each fixture has N free nodes, one ground, a 1 S shunt per node and a 0.25 S branch between adjacent nodes. An injection at node 1 varies from 0 to 1 A in 101 samples. All runs use the same limits and cold-start each operating point. The sum of the 101 node-1 voltages printed as 41.835569799160 V in all nine size/mode combinations.

| Free nodes | Branches | Previous one-shot × 101 | Current one-shot × 101 | Prepare once + 101 solves | Previous → prepared peak heap | Previous → prepared allocations |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 | 199 | 1.455 ms | 1.322 ms | 1.158 ms | 13,989 → 11,301 B | 11,312 → 12 |
| 1,000 | 1,999 | 13.649 ms | 12.958 ms | 11.278 ms | 137,257 → 113,001 B | 102,515 → 12 |
| 10,000 | 19,999 | 127.029 ms | 128.042 ms | 114.339 ms | 1,421,137 → 1,130,001 B | 1,011,919 → 12 |

The previous one-shot results use commit `8a6a225`. Each timing is the median of nine separate executions after one warm-up. Mode order rotated between repetitions. Timing starts after fixture construction, includes preparation and every solve, and ends before formatting output. At 10,000 nodes preparation reuse reduced elapsed time by 10.0% and peak live heap by 20.5% relative to the previous revision. Rebuilding for each sample on that fixture took 0.8% longer than the previous revision.

A separate instrumented run counted successful Rust system-allocator allocations and reallocations and tracked the peak live requested bytes above the pre-solve fixture. All 12 allocations in prepared mode occurred during construction at every size. The 101-point advance loop performed zero allocations. A second probe around the public circuit API confirmed zero post-preparation allocations while reading all 101 named divider points. These heap figures exclude input construction, allocator bookkeeping and process RSS. The performance figures apply to this grounded chain. The indefinite-matrix benchmark in the [release checklist](checklist.md#v02) still requires a general MNA backend.

Run the checked-in fixture with either `prepared` or `rebuild`. The three output fields are free nodes, elapsed milliseconds and the voltage checksum.

```sh
cargo build --release --locked --offline --example transfer_profile
target/release/examples/transfer_profile 10000 prepared
target/release/examples/transfer_profile 10000 rebuild
cargo run --locked --offline --bin electra -- dc examples/transfer.cir
```
