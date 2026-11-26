# Direct AC node voltages

Run the opt-in Leto adapter from the Electra workspace root:

```sh
cargo run --locked --release --manifest-path integrations/leto-direct/Cargo.toml --bin electra-ac-solve -- examples/ac-bridge.cir 0.15915494309189535
```

The command takes one flat R/C/L deck with ground-referenced AC voltage sources and one frequency in Hz. Standard output contains CSV node labels and rectangular voltages in V. Standard error reports the largest free-node physical current imbalance in A. The bridge command gives 0.45901639 − j0.04918033 V at node a and 0.36065574 + j0.03278689 V at node b.

Use `electra_leto_direct::ac::solve` with a parsed `AcDeck` to obtain typed voltages in original node order. `AcDeck::branch_currents` visits each passive element's signed current from its first terminal to its second, along with its original line and node indices. Each call to solve assembles and factors one frequency. The deck can be reused across frequencies.

## Algorithm and limits

Matrix export and solve use the same compensated sparse assembly. Each nonzero free-node complex row is divided by the largest absolute real or imaginary coefficient component in that row. An entry a + jb becomes the real block [[a, −b], [b, a]] for interleaved voltage quadratures. Connectivity is checked using nonzero off-diagonals and the net admittance to prescribed nodes. Traversal uses the ordered sparse row index. Prescribed nodes are eliminated first, so channels can share a voltage drive or ground and still be independent. Each connected component is factored and released separately, including for zero excitation. At 128 free nodes in one component the order is 256 and each dense f64 matrix occupies 512 KiB. Factorization entails a working copy. Storage also includes sparse coefficients, vectors and provider overhead. A component exceeding 128 free nodes is rejected during traversal, before allocating its dense matrix. Total deck storage also grows with the number of nodes and sparse entries.

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

The regression suite also checks interleaved node numbering and a 129-node capacitive chain that separates into independent rows at DC and becomes one oversized connected component at positive frequency.
