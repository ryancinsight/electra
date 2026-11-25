# Pivoted direct DC analysis

Use the optional Leto adapter to solve a linear DC bias or sweep of up to 256 non-reference unknowns. Each non-reference node contributes one voltage and each voltage-defined branch contributes one current. The adapter assembles Electra's dimensionless MNA coefficients directly into a row-major buffer, factors it once with partial pivoting, and reuses the factor, RHS, solution and probe buffers for every excitation. Electra checks the original KCL and voltage constraints in Aequitas units at each point. The core CLI continues to choose reduction for independent sources and Athena GMRES for controlled sources.

```sh
cargo build --release --locked --manifest-path integrations/leto-direct/Cargo.toml --target-dir target --bin qualify
target/release/qualify op integrations/leto-direct/fixtures/pivot.cir
target/release/qualify dc examples/controlled-sources.cir --solver direct
```

The Rust API offers `operating_point_direct::<PivotedLu>`, `dc_sweep_direct::<PivotedLu>` and `sweep_source_direct::<PivotedLu>`. A provider receives a borrowed normalized matrix and supplies a concrete factor implementing `DirectFactor`. Stamp contributions that share a coordinate are additive. The ground slot stays inside Electra. An oversized system returns `MnaError::DirectOrder` before operator or factor allocation. Numerical and physical failures terminate a sweep at the identified excitation.

## Accuracy and pivot failures

The independent oracle assembles the physical equations using rational arithmetic. Each family has 32 seeded circuits, two device orders, flat and hierarchical forms, a bias point and five sweep points. At each of the two tolerance settings the published dense solver and default dispatcher passed 28,416 independent-source and 18,432 controlled-source comparisons. All fixture orders are small enough for Leto's default dense branch. The largest differences across all printed probes were 4.832e−12 V and 4.992e−13 A. Comparison limits were 1e−9 relative plus 1e−10 V or 1e−12 A absolute.

Forcing a natural-order sparse attempt on the 128 distinct flat bias orderings distinguishes pivot-free solves, handoffs to dense LU and false singularities. The corrected provider passed all 46,848 comparisons at each tolerance with sparse-first dispatch.

| Provider and circuit family | Sparse success | Dense handoff | False singular |
| --- | ---: | ---: | ---: |
| Published 0.52.0, independent | 0 | 59 | 5 |
| Published 0.52.0, controlled | 16 | 45 | 3 |
| Corrected, independent | 0 | 64 | 0 |
| Corrected, controlled | 16 | 48 | 0 |

The [six-device reproducer](../integrations/leto-direct/fixtures/pivot.cir) has a unique solution. Nodes a and tap are −7/3 V, mid is 8/3 V, and b is 14/3 V. Adding 27 separate 1 Ω resistors from new nodes to ground produces 33 unknowns and 43 stored entries. The default 0.52.0 dispatcher chooses sparse factorization and incorrectly reports a zero pivot at column 32. In the minimized 3 × 3 matrix below, the determinant is +1. A required row exchange invalidates the natural symbolic fill pattern before column 2 is reached.

$$
A=\begin{bmatrix}0&1&0\\0&1&1\\1&0&0\end{bmatrix},\qquad
A\begin{bmatrix}1\\2\\3\end{bmatrix}=\begin{bmatrix}2\\5\\1\end{bmatrix}.
$$

The provider correction returns the pivot-required diagnostic on the first required exchange and lets Leto retry with dense partial pivoting. It also limits the pivot search to reachable rows and removes the inverse-permutation workspace. The Electra adapter uses the published dense implementation directly. Reassess the sparse dispatcher after a published provider update. [Leto regression and fix](https://github.com/ryancinsight/leto/pull/443)

## Sweep cost and operating limit

Measurements used Rust 1.95.0 in release mode on macOS arm64. Each four-device cell has a grounded supply, two resistors and a floating voltage source. The sweep drives the first supply through 101 points from 0 to 10 V. Times are medians of nine uninstrumented executions after warm-up. Peak heap is the maximum of live requested bytes above the parsed circuit, measured separately with a counting system allocator. It excludes parsing, allocator overhead and process RSS. Setup includes operator construction and factorization. Each successful row had zero allocations while advancing the sweep.

| Elements | Unknowns | Direct setup / total | Direct peak / setup allocations | Published dispatcher setup / total | Dispatcher peak / setup allocations |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 20 | 25 | 8 / 52 µs | 11,128 B / 14 | 15 / 58 µs | 13,400 B / 52 |
| 100 | 125 | 110 / 470 µs | 255,608 B / 14 | 142 / 484 µs | 275,700 B / 81 |
| 204 | 255 | 932 / 2,070 µs | 1,051,832 B / 14 | 1,032 / 2,184 µs | 1,092,420 B / 86 |
| 1,000 | 1,250 | Order limit | — | 69,054 / 91,527 µs | 37,776,076 B / 117 |
| 10,000 | 12,500 | Order limit | — | Order limit (2,048) | — |

The adapter's 256-unknown ceiling bounds the two dense square buffers at 1 MiB combined. Pivots, compiled stamps, physical rank checks and probe buffers add circuit-dependent storage. Its regression tests exercise orders 256 and 257. At 1,000 elements the matrix-free Athena fixture previously used 499,192 B and took 0.939 ms. These repeated cells favor Athena's affine sweep prediction. The runtime and allocation evidence supports explicit bounded direct solves and keeping larger problems on the existing solvers. [Athena measurements](modified-nodal-analysis.md)

A second fixture isolates a successful sparse solve. It has one grounded 1 Ω resistor per node, a current source from `p0` to ground, and the same 101-point sweep. At 2,048 unknowns, median setup fell from 2,043 µs on published Leto to 110 µs with the reachable-row pivot search. Total sweep time fell from 4,270 µs to 2,309 µs in nine alternating runs. Both versions peaked at 393,328 B. The sparse triangular solve allocated two vectors per excitation, or 202 allocations for the sweep. This case exercises a different path from the dense handoffs in the table. Revisit sparse solve workspace reuse before adding it to the Electra CLI.

The core workspace locks 15 packages, including nine inherited third-party versions. The isolated qualification workspace locks 108, including 83 inherited third-party versions under the Atlas providers. All direct declarations refer to Atlas repositories. Keeping Leto in a separately locked integration makes its cost explicit.

## Replay

```sh
cargo test --locked --workspace
cargo test --release --locked --manifest-path integrations/leto-direct/Cargo.toml --target-dir target
python3 scripts/check-dependencies.py --leto-direct
python3 integrations/leto-direct/scripts/qualify-pivots.py target/release/qualify
cargo build --release --locked --example mna_profile
python3 integrations/leto-direct/scripts/profile-cells.py target/release/qualify
```

Run the exact oracle for each combination of deck layout, analysis and source family. Repeat with `solver=dispatch` for Leto's default selection and `solver=sparse-first` for the corrected provider. Published 0.52.0 reproduces the false singularities in the sparse-first run.

```sh
solver=direct
for layout in '' --hierarchical; do
  for analysis in '' --sweep; do
    for family in '' --controlled; do
      python3 scripts/verify-dc-mna.py target/release/qualify --solver "$solver" $layout $analysis $family || exit 1
    done
  done
done
```

To replay the pivot-free sparse timing, generate a diagonal network and run the same command with the published and corrected qualification binaries.

```sh
python3 -c 'print("Iload p0 0 1", *(f"R{i} p{i} 0 1" for i in range(2048)), ".dc Iload 0 10 0.1", sep="\n")' > /tmp/electra-diagonal.cir
target/release/qualify profile /tmp/electra-diagonal.cir --solver dispatch
```
