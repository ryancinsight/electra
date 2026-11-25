# Modified nodal DC analysis

Select `--solver mna` on an operating point or a single-source transfer sweep. MNA supports resistors, independent sources, floating and zero-volt probes, and linear E/G/F/H sources. The CLI selects MNA for dependent sources and reduction for independent-source decks. Reduction uses boundary elimination or supernodes. [Controlled-source equations and qualification](controlled-sources.md) Both solvers accept hierarchical decks, included files and fully qualified source names.

```sh
cargo run --release --offline --locked --bin electra -- op examples/floating-source.cir --solver mna
cargo run --release --offline --locked --bin electra -- dc examples/hierarchical-divider.cir --solver mna --atol-amps 1e-14 --atol-volts 1e-12 --rtol 1e-13
```

In Rust, use `Circuit::operating_point_mna`, `dc_sweep_mna`, or `sweep_source_mna` with an Aequitas-typed range. Each sweep sample borrows its voltage and signed-current probes until the next advance. `MnaPoint` reports GMRES iterations and the largest source voltage error. Its shared `probes` record holds the largest current imbalance. The MNA defaults are 1e−12 A, 1e−12 V, 1e−10 relative and 100,000 Arnoldi iterations per point. Set the budget with `--max-iterations`.

## Equations, units and acceptance

Let $B$ have +1 at a voltage source's positive terminal and −1 at its negative terminal. With reference voltage fixed, the equations for node potentials $v$ and passive-sign voltage-source currents $i$ are

$$
\begin{bmatrix}G & B \\ B^{T} & 0\end{bmatrix}
\begin{bmatrix}v \\ i\end{bmatrix}
=
\begin{bmatrix}j \\ e\end{bmatrix}.
$$

The upper rows express current conservation in amperes. Each lower row imposes an oriented voltage difference in volts. A single ideal reference-connected voltage source yields a nonsingular two-by-two block with zero diagonal entries. A grounded resistor and source network can therefore require an indefinite solver. [Ho, Ruehli and Brennan](https://doi.org/10.1109/TCS.1975.1084079)

Electra applies these blocks directly from the resistor and source terminal lists. A pinned zero slot preserves the circuit's node indices. Voltages use a 1 V reference. Each non-reference node starts with the largest incident resistor conductance times 1 V as its current scale. An ideal source takes the larger of its endpoint scales, with a 1 A fallback for a source-only connection. Each node then takes the maximum of its resistor and incident source scales. Dividing each KCL row by that scale and expressing each source current in its own scale produces a dimensionless, generally nonsymmetric operator. Electra rejects a normalization that would erase a nonzero coefficient or produce a nonfinite value.

Athena owns restarted GMRES. Electra supplies the circuit operator and a host implementation of its vector contract. Rust specializes the recurrence for borrowed slices and a restart width of 16. Basis memory is linear in the circuit order at that fixed width. Dot products use pairwise accumulation. For the 64 bias decks below, width 8 took 7,012 Arnoldi steps versus 832 at width 16. [Saad and Schultz](https://doi.org/10.1137/0907058)

Every successful point must satisfy both of the following physical checks after Athena reports numerical convergence.

| Equation | Acceptance limit |
| --- | --- |
| KCL at every original node, including ground | Absolute current tolerance plus relative tolerance times the sum of absolute incident currents |
| Voltage drop across each ideal source | Absolute voltage tolerance plus relative tolerance times the absolute imposed voltage |

The solver also checks finite probes and detects a nonzero independent-source voltage that vanishes in the returned node potentials. If the global norm passes while a physical row fails, the solver can solve the residual equation once more within the original iteration budget. The correction uses its own right-hand-side scale. A 0.125 V source and 2 Ω load beside an independent 1e16 V rail exercise this case. A 1 V drop on a 1e16 V common-mode potential exercises loss of resolution and produces a diagnostic.

Both backends use compensated summation for fixed current injections. The reduced solver also compensates current recovery through floating voltage-source trees. Tests preserve a 1 A bias between opposing 1e20 A sources, recover a 0.5 A probe current and retain the bias when sweeping a canceling source.

## Independent qualification

The Python reference assembles a full matrix in exact rational arithmetic, solves it by pivoted elimination and compares every reported probe. It checks finite values, complete probe inventories and excitation order. It runs 32 seeded networks in both element orders. Each network is also encoded with nested subcircuits. Sweep qualification alternates between voltage and current drives and between increasing and decreasing ranges. Each of its five samples gets a separate exact solution.

| MNA case | Probe comparisons per encoding and tolerance setting | Maximum printed voltage error | Maximum printed current error | Outside probe gate |
| --- | ---: | ---: | ---: | ---: |
| Nominal operating point | 2,368 | 4.796e−12 V | 4.992e−13 A | 0 |
| Five-point transfer | 11,840 | 4.832e−12 V | 5.007e−13 A | 0 |

Both flat and hierarchical decks met the probe gate at default and qualification settings. The gate is 1e−9 relative plus 1e−10 V or 1e−12 A absolute. Comparisons include CLI rounding. The reduced solver also passed at the qualification settings. At default settings it exceeded the same per-probe gate for 12 bias probes and 61 sweep probes per encoding.

Singular-case checks cover a floating component with zero excitation and incompatible or redundant ideal-source loops. A budget-limited run on a nonsingular divider reports Krylov termination. GMRES performs no LU pivot selections. Direct factorization and pivot diagnostics are tracked separately on the [checklist](checklist.md#v02).

```sh
cargo test --offline --locked -p electra-circuit --test mna
cargo build --release --offline --locked --bin electra --example mna_profile
python3 scripts/verify-dc-mna.py --solver mna
python3 scripts/verify-dc-mna.py --solver mna --hierarchical --sweep
```

## Transfer timing and memory

The fixture contains four physical elements per cell, including a floating voltage source. One rail is swept from 0 to 10 V in 101 points. The benchmark checks the swept voltage and analytical currents in the driven cell and a stationary cell. Preparation and solving are timed after parsing. The MNA sweep uses the first two checked solutions to predict later points on the affine response. Athena recomputes the normalized residual at each prediction and Electra checks every physical equation before returning the sample.

Measured November 24, 2026 on macOS arm64, Rust 1.95.0, release mode. Times are medians of nine executions after a warm-up, rotating mode order. The cold-start comparison used the same implementation with its solution vector cleared immediately before each solve.

| Physical elements | Reduced sweep | Cold-start MNA sweep | MNA with prediction | MNA preparation, included | Cold / predicted Arnoldi steps | Max MNA KCL / source voltage residual |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 | 0.111 ms | 0.374 ms | 0.109 ms | 0.009 ms | 505 / 32 | 3.76e−15 A / 5.06e−13 V |
| 1,000 | 0.999 ms | 3.475 ms | 0.939 ms | 0.040 ms | 505 / 10 | 3.31e−14 A / 5.29e−13 V |
| 10,000 | 10.499 ms | 34.522 ms | 8.721 ms | 0.333 ms | 505 / 10 | 3.31e−13 A / 1.76e−13 V |

| Physical elements | Reduced peak live heap | MNA peak live heap | Reduced / MNA preparation allocations | Allocations while advancing either sweep |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 9,746 B | 52,792 B | 18 / 22 | 0 |
| 1,000 | 96,371 B | 499,192 B | 18 / 22 | 0 |
| 10,000 | 962,621 B | 4,963,192 B | 18 / 22 | 0 |

A separate system-allocator counter tracked peak additional live requested bytes and allocation/reallocation calls above the parsed circuit. The MNA basis accounts for much of the additional storage. A point that requires residual correction can allocate two extra work vectors once and reuse them. The table characterizes this repeated-cell fixture. Process RSS, allocator bookkeeping and deck parsing are excluded.

```sh
target/release/examples/mna_profile 10000 mna
target/release/examples/mna_profile 10000 reduced
```

## Provider scope

`athena-core 0.4.0` adds one Atlas package and reuses the locked `eunomia 0.12.1`. The other providers are `aequitas 0.5.0` and `eunomia-derive 0.12.1`. The nine inherited third-party versions remain `bytemuck 1.25.2`, `bytemuck_derive 1.12.1`, `libm 0.2.16`, `proc-macro2 1.0.107`, `quote 1.0.47`, `syn 2.0.119`, `syn 3.0.10`, `typenum 1.20.1` and `unicode-ident 1.0.27`. A separate dependency-resolution trial with `athena-leto 0.5.0` added 95 locked packages over the existing lockfile. Electra links only the core recurrence for this milestone. [Dependency policy](architecture.md#atlas-only-dependency-rule)

The optional Leto adapter shares the normalized stamps, drive and physical checks with the iterative path. [Pivoted LU qualification](pivoted-direct.md)
