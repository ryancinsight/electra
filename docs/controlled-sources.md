# Linear controlled sources in DC analysis

Electra accepts all four linear dependent-source cards in operating points and single-source DC sweeps. On a deck containing any of these cards the CLI selects modified nodal analysis (MNA). In Rust, select `operating_point_mna`, `dc_sweep_mna` or `sweep_source_mna`. A generic caller can inspect `Circuit::requires_mna()` before choosing the backend. [Runnable example](../examples/controlled-sources.cir)

| Card | Enforced output from p to n | Gain parameter |
| --- | --- | --- |
| `Ename p n a b k` | Voltage equals k × (Va − Vb) | Dimensionless voltage gain |
| `Gname p n a b g` | Current equals g × (Va − Vb) | Conductance in siemens |
| `Fname p n Vprobe k` | Current equals k × I(Vprobe) | Dimensionless current gain |
| `Hname p n Vprobe r` | Voltage equals r × I(Vprobe) | Transresistance in ohms |

Positive sensing current flows from the named branch’s first terminal to its second. The controller can be a V, E or H voltage-defined branch, including a zero-volt probe. Names are case-insensitive and forward references are valid. A controller inside a subcircuit resolves within that instance. Pass voltage-sensing nodes through formal pins. A qualified branch name identifies an individual output probe, and a qualified independent source can drive a sweep. [ngspice linear dependent-source equations](https://nmg.gitlab.io/ngspice-manual/voltageandcurrentsources/lineardependentsources.html)

Primitive literals use the element’s base unit and optional scale factor. Model declarations take an explicit unit, such as `gain=-2x`, `gm=1mS` or `rm=2kOhm`. Uppercase S denotes siemens and lowercase s denotes seconds. Zero and negative gains are supported. Tying the two voltage-sensing pins together gives zero control voltage. The implementation covers constant linear coefficients and typed parameter references. Nonlinear expressions, source multipliers and sweeping a gain are outside this milestone.

## Analytical checks

In the example a 2 V input drives a 1 kΩ resistor and a series zero-volt probe. The probe measures +2 mA. The load voltages are −4 V for E, −4 V for G, +6 V for F and +4 V for H. The Rust tests check every branch current, including the current through each controlled voltage output, at nominal bias and at five drive levels through zero. They also exercise repeated private controllers and forward references to dependent branches.

A second test closes a voltage amplifier with an open-loop gain of 100,000 around an equal-resistor feedback divider. For a 0.25 V input, the output is 25,000/50,001 V. Scaling both sides of each controlled voltage equation by the larger of one and its normalized gain prevents the gain coefficient from dominating the GMRES residual. Final acceptance checks the original voltage law in volts and the original node currents in amperes.

## Rank and storage

A zero drive can yield a zero residual even when a feedback circuit has infinitely many operating points. Controlled circuits therefore undergo a sparse numerical rank check at preparation. The same statically dispatched stamps assemble the rank-check rows and apply the matrix-free GMRES operator. The ground potential is prescribed. Each accepted pivot must exceed 64 machine epsilons times the row’s accumulated cancellation scale. The solver rejects singular or numerically unresolved feedback, including unity-gain loops and exactly cancelling conductances. A self-sensing transconductance can supply the entire DC path to ground. For equal-magnitude pivot candidates the check prefers the natural diagonal. A 257-resistor ladder verifies that forward, reverse and interleaved device orders all accept the same grounded zero-bias solution.

The check stores input coefficients and pivot rows in contiguous buffers and reuses one scratch row. Its temporary storage is released before GMRES allocates its basis. Fill during elimination depends on topology and ordering. A numerically ill-conditioned circuit can fail this rank check. This procedure does not compute a direct solution or distinguish incompatible excitation from consistent underdetermination in a general controlled loop. Independent ideal-source loops retain their specific parse-time diagnostics.

## Reference and performance evidence

The Python reference stamps a dense matrix in exact rational arithmetic and solves it by pivoted elimination. The controlled-source fixture uses 32 seeded feedback networks with signed gains, varied port orientations and voltage-branch sensing. Two card orders and two encodings exercise flat and nested decks. Five-point sweeps alternate independent voltage and current drives and ascending and descending order. The probe gate is 1e−9 relative plus 1e−10 V or 1e−12 A absolute.

| Case | Comparisons per encoding and tolerance setting | Largest printed voltage error | Largest printed current error |
| --- | ---: | ---: | ---: |
| Operating point | 1,536 | 4.916e−13 V | 4.807e−13 A |
| Five-point sweep | 7,680 | 4.238e−12 V | 4.969e−13 A |

All 18,432 comparisons across both encodings met the gate at each of the default and qualification tolerances. The preceding independent-source suite also passed all 28,416 comparisons per setting.

The resource fixture repeats ten-element subcircuits with all four controlled sources, including a voltage feedback loop. One instance’s supply sweeps from 0 to 10 V in 101 samples. Each point checks its four output voltages and supply current, and the supply current of a stationary cell when present. Measurements below are local macOS arm64 release builds with Rust 1.95. Times include solver preparation and exclude parsing. Each median uses nine runs after warm-up. A separate system-allocator counter measured requested heap above the parsed circuit.

| Elements | Preparation | Complete sweep | Peak live solver heap | Setup allocation calls | Calls while advancing |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 100 | 0.014 ms | 0.124 ms | 38,472 B | 34 | 0 |
| 1,000 | 0.058 ms | 0.896 ms | 355,992 B | 37 | 0 |
| 10,000 | 0.445 ms | 12.601 ms | 3,531,192 B | 41 | 0 |

At 10,000 elements the initial per-row rank storage took 8,043 setup allocation calls. Packed storage reduced this to 41. The maximum original-node imbalance over the sweep was 1.55e−12 A and the maximum voltage-source discrepancy was 1.48e−12 V. Both met their per-equation absolute plus relative limits. These timings and memory results describe the repeated-cell topology.

```sh
cargo test --locked --offline --test controlled
cargo build --release --locked --offline --bin electra --example mna_profile
target/release/electra dc examples/controlled-sources.cir
python3 -B scripts/verify-dc-mna.py --solver mna --controlled
python3 -B scripts/verify-dc-mna.py --solver mna --controlled --hierarchical --sweep
target/release/examples/mna_profile 10000 controlled
```
