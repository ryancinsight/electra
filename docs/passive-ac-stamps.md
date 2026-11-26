# Passive AC nodal assembly

The `electra_ac::nodal` API accepts two-terminal R, C and L devices between arbitrary pairs of numbered nodes. It uses exp(+jωt). The branch admittances in siemens are 1/R, j2πfC and −j/(2πfL). The branch computes the resistive reciprocal once at construction. Capacitors open at DC. An inductor between distinct nodes at DC requires an ideal-short constraint and returns a diagnostic.

Pass a boundary slice of optional complex voltages, indexed by node. For two free endpoints the shared Electra network kernel emits +Y on each diagonal and −Y on each off-diagonal. With one prescribed endpoint it adds Y to the free diagonal and Y times the prescribed voltage to the free RHS. Repeated callbacks add to the same entry. Identical endpoints have zero net contribution. Fixed rows have no free-node equation.

The caller supplies statically dispatched matrix and RHS callbacks. The API allocates no matrix or scratch buffer. It validates each branch before emitting its first term, including checking for a nonzero source-current quadrature lost to underflow. The caller owns accumulation, source stamping, checks on accumulated coefficients, and the solve. Clearing or discarding a partial assembly after a later branch fails is the caller's responsibility. The crate documentation contains an executable one-node RC example.

## Verification and cost

The bridge fixture uses a capacitor between two free nodes, four resistors to the source and ground, and a complex drive of 1 + 2j V. At unit angular frequency the determinant is 6 + 5j. Both analytically calculated node voltages satisfy the assembled equations within 2 × 10⁻¹⁵ A in either branch orientation. Separate tests exercise inductive source phase, invalid devices and endpoints, zero-frequency behavior, and representable subnormal admittances.

The release fixture stamps a chain alternating R, C and L into three complex matrix bands and a complex RHS, clearing and reusing the buffers at each of 101 frequencies. The table reports the median of three local Linux x86-64 runs with Rust 1.95.0. Buffer bytes count element, boundary, band and RHS slice payloads. Timing includes clearing and assembly and excludes setup, solving and I/O.

| Nodes | Branch stamps | Median assembly time | Buffer bytes |
| ---: | ---: | ---: | ---: |
| 100 | 9,999 | 0.147 ms | 11,968 |
| 1,000 | 100,899 | 1.648 ms | 119,968 |
| 10,000 | 1,009,899 | 16.171 ms | 1,199,968 |

```sh
cargo test --locked -p electra-ac --test nodal
cargo test --locked -p electra-ac --doc
cargo run --locked --release -p electra-ac --example profile-stamps -- 10000
```

## Flat AC netlists

Run `electra ac-matrix examples/ac-bridge.cir 1000` to export the reduced system Yv = b at 1,000 Hz. The parser accepts one card per line, whole-line asterisk comments, optional final `.end`, and these cards:

| Card | Quantity and convention |
| --- | --- |
| `Rname a b value` | Resistance in Ω |
| `Cname a b value` | Capacitance in F |
| `Lname a b value` | Inductance in H |
| `Vname p n AC magnitude [phase]` | V(p) − V(n), voltage magnitude in V and optional phase in degrees. Exactly one terminal must be ground (`0`). |

The same decimal, scientific, and suffix parser serves AC and DC. The CLI frequency argument takes numeric Hz. Ground has index zero. The parser assigns other indices in first-appearance order, folds ASCII case for lookup and retains the original node spelling for display. Each driven node accepts one source. At least one passive branch is required. Unsupported cards, floating voltage sources, duplicate element names and extra fields produce line-numbered errors.

The CSV header is `kind,row,column,label,real,imag`. A `fixed` row holds a known complex voltage in V. A `free` row identifies an unknown. `Y` rows hold complex admittances in S and `rhs` rows hold injected currents in A. The indices are the original node indices. Solve over the free rows and columns. Duplicate contributions use compensated summation for each quadrature and exact zero coefficients are omitted. A floating island or resonant singularity can therefore appear in a valid matrix export. Rank checks belong to the solve stage.

For sweeps, call `electra_circuit::parse_ac` once and reuse `AcDeck::stamp` at each frequency. The typed callbacks reuse the caller's storage. The CLI uses an ordered sparse map for coefficients and a node-sized RHS, and starts writing CSV only after successful assembly. The bridge CLI test reads the actual exported coefficients and checks both nodal residuals against the analytical solution at unit angular frequency.

On Linux x86-64 with Rust 1.95.0, a release CLI run at 1 kHz took median 5.15, 21.47 and 209.42 ms for 1,000, 10,000 and 100,000 RC sections respectively. Each section adds a 1 kΩ series resistor and 1 µF shunt capacitor to the preceding node, starting at a prescribed 1 V source. Measurements cover process startup, file reading, parsing, assembly and CSV formatting to the null device. Each median uses five runs on a local file.
