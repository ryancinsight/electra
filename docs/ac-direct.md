# Direct AC node voltages

Run the opt-in Leto adapter from the Electra workspace root:

```sh
cargo run --locked --release --manifest-path integrations/leto-direct/Cargo.toml --bin electra-ac-solve -- examples/ac-bridge.cir 0.15915494309189535
```

The command takes one flat R/C/L deck with ground-referenced AC voltage sources and one frequency in Hz. Standard output contains CSV node labels and rectangular voltages in V. Standard error reports the largest free-node physical current imbalance in A. The bridge command gives 0.45901639 − j0.04918033 V at node a and 0.36065574 + j0.03278689 V at node b.

Use `electra_leto_direct::ac::solve` with a parsed `AcDeck` to obtain typed voltages in original node order. `AcDeck::branch_currents` visits each passive element's signed current from its first terminal to its second, along with its original line and node indices. Each call to solve assembles and factors one frequency. The deck can be reused across frequencies.

## Algorithm and limits

Matrix export and solve use the same compensated sparse assembly. Each nonzero free-node complex row is divided by the largest absolute real or imaginary coefficient component in that row. An entry a + jb becomes the real block [[a, −b], [b, a]] for interleaved voltage quadratures. Connectivity is checked using nonzero off-diagonals and the net admittance to prescribed nodes. Traversal uses the ordered sparse row index. Prescribed nodes are eliminated first, so channels can share a voltage drive or ground and still be independent. Each connected component is factored and released separately, including for zero excitation. At 128 free nodes in one component the order is 256 and each dense f64 matrix occupies 512 KiB. Factorization entails a working copy. Storage also includes sparse coefficients, vectors and provider overhead. Larger acyclic components use the tree solver described below. Large cyclic components use low-degree elimination with a bounded pivoted remainder. Total deck storage also grows with the number of nodes and sparse entries.

A solution must pass both checks:

- In each original, unscaled row, the magnitude of Yv − b must be at most 10⁻¹⁰ times the sum of the magnitudes of the individual Yv terms and b.
- Branch currents are recomputed as Y times the original voltage difference. At every free node, the magnitude of the oriented sum must be at most 10⁻¹² A plus 10⁻¹⁰ times the sum of branch-current magnitudes. This check catches a lost voltage drop across a low-resistance branch even when the assembled equation is satisfied to floating-point precision.

Range errors, unanchored components, zero rows, provider factorization failures and excessive residuals return errors before voltage CSV output. These checks assess current balance. Forward voltage accuracy near an ill-conditioned resonance depends on conditioning. A DC inductor between distinct nodes still requires an ideal-short constraint.

## Independent checks and measured cost

Tests cover the analytical bridge, the loaded two-section ladder at DC and five positive frequencies, an RL divider with a quadrature source, the physical current orientation, a floating ring, exact LC cancellation, a lost voltage drop, fixed-only nodes, and independent rows whose admittances differ by 300 orders of magnitude.

The separate NumPy check builds a complex incidence matrix from the generated element list and solves its reduced equations directly. It checks every free voltage. Each timing below is the median of seven Linux x86-64 release CLI runs at 1 kHz, including process startup, parsing, factorization, residual checks and CSV output.

| Free nodes | Median elapsed | Max voltage difference | Max branch KCL residual |
| ---: | ---: | ---: | ---: |
| 4 | 1.42 ms | 3.35 × 10⁻¹⁶ V | 1.23 × 10⁻¹⁷ A |
| 32 | 1.51 ms | 1.12 × 10⁻¹⁵ V | 2.37 × 10⁻¹⁷ A |
| 128 | 3.37 ms | 7.58 × 10⁻¹⁶ V | 1.54 × 10⁻¹⁷ A |

```sh
cargo test --locked --manifest-path integrations/leto-direct/Cargo.toml
cargo build --release --locked --manifest-path integrations/leto-direct/Cargo.toml --bin electra-ac-solve
python3 integrations/leto-direct/scripts/verify-ac.py integrations/leto-direct/target/release/electra-ac-solve
```

## Independent channels

A release run with 4,096 independent RC channels solved 8,192 free voltages in a median 14.38 ms across seven CLI executions on Linux x86-64. Each channel consists of two 1 Ω series resistors from a common 1 V drive, followed by a 1 F capacitor to ground at unit angular frequency. All CSV voltages matched the analytical values 0.6 − j0.2 V and 0.2 − j0.4 V to the precision of the parsed f64 reference. Maximum branch KCL residual was 5.56 × 10⁻¹⁷ A. The largest real factor was 4 × 4. The dense input accounted for 128 bytes per active factor, excluding the factorization working copy and sparse deck storage.

The regression suite also checks interleaved node numbering and a 129-node capacitive chain that separates into independent rows at DC and becomes one connected tree at positive frequency.

## Connected trees above 128 free nodes

The solver inspects the reduced admittance graph at the requested frequency. Each non-root node in an acyclic component has exactly one earlier neighbor in breadth-first order. The solver eliminates leaves toward the root, then substitutes from the root outward. Each elimination updates only the parent diagonal and right-hand side. The row-normalized coefficients from parent to child and child to parent are stored separately. Diagonal and current updates use compensated complex summation.

For a tree of n free nodes the factor workspace holds n row records, and both elimination and substitution take O(n) arithmetic operations. Sparse assembly, ordered-map lookups, parsing and CSV output have separate costs. A zero elimination pivot can occur even on a nonsingular resonant tree. An unusable leaf pivot transfers the component to sparse reduction, which can enforce a zero-diagonal leaf constraint directly. Both original-equation and physical branch KCL checks run before voltage output.

The independent fixture builds grounded binary RLC trees at 1 kHz and compares every free-node voltage with SciPy's sparse direct solver. Elapsed times are medians of seven release CLI executions, including parsing and CSV output. The error columns give maximum magnitudes.

| Connected free nodes | CLI elapsed | Voltage difference | Physical KCL residual |
| ---: | ---: | ---: | ---: |
| 129 | 1.80 ms | 6.16 × 10⁻¹⁵ V | 1.66 × 10⁻¹⁶ A |
| 1,023 | 3.66 ms | 2.12 × 10⁻¹⁵ V | 8.97 × 10⁻¹⁷ A |
| 8,191 | 22.82 ms | 2.78 × 10⁻¹⁶ V | 8.50 × 10⁻¹⁷ A |

Run `python3 integrations/leto-direct/scripts/verify-tree.py integrations/leto-direct/target/release/electra-ac-solve` after building the release binary. Rust regressions also cover an analytical loaded star, frequency-dependent connectivity, a cyclic network and a resonant leaf constraint.

## Cyclic components and resonant leaves

At each frequency, the solver can reduce a large component with cycles by removing rows with one or two remaining neighbors. A two-neighbor elimination introduces at most one undirected edge. The solver records the row for reverse substitution. Diagonal, current and repeated edge updates use compensated complex summation. A scalar pivot smaller than half of an incoming coefficient in the largest-quadrature norm is deferred. For a zero-diagonal leaf with nonzero coupling in both directions, the leaf equation directly determines its neighbor’s voltage. The neighbor equation then recovers the leaf voltage during reverse substitution.

A resonant leaf constraint can disconnect the remaining equations. Electra traverses the surviving edges and applies Leto pivoted LU separately to each connected block of up to 128 complex unknowns. Dense factor storage depends on the largest block. The regression with forty four-node blocks checks all 160 remaining voltages against the analytical divider solution. Each dense input uses 512 bytes (8 × 8 real entries). For larger blocks Electra measures the half-bandwidth in breadth-first node order. At half-bandwidth w ≤ 32, complex Gaussian elimination searches the active column for a row pivot and stores sparse rows through back substitution. A row swap can extend the upper band to 2w, so factor storage grows as O(nw) for n unknowns. Wider blocks return an error before factor allocation. The 130-node circulant regression checks every alternating voltage against its analytical value (0.4j or 0.6j V). A small residual alone does not guarantee voltage accuracy near singular resonance. Both the unscaled equations and the original physical branch currents are checked before results are emitted.

The seeded ring fixture drives two nodes through resistors with different source phases and includes series R/L elements and shunt capacitors. SciPy independently assembles and solves the complex sparse equations. Median times include CLI startup, parsing, solution, checks and CSV output across seven release runs.

| Ring nodes | Median elapsed | Maximum voltage difference | Maximum physical KCL residual |
| ---: | ---: | ---: | ---: |
| 129 | 2.45 ms | 3.53 × 10⁻¹⁵ V | 1.87 × 10⁻¹⁶ A |
| 1,023 | 7.41 ms | 3.41 × 10⁻¹⁵ V | 1.80 × 10⁻¹⁶ A |
| 8,191 | 30.37 ms | 3.87 × 10⁻¹⁵ V | 1.51 × 10⁻¹⁶ A |

Reproduce with `python3 integrations/leto-direct/scripts/verify-tree.py integrations/leto-direct/target/release/electra-ac-solve --ring`. Rust fixtures check both arms of a driven resistor ring, a subdivided multicycle network against its lumped equivalent, a resonant leaf with a known voltage, and rejection of a 130-node degree-four core.
