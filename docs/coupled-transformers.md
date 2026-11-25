# Coupled windings and the ideal transformer limit

Electra computes secondary voltage gain and primary input impedance under an ideal sinusoidal primary voltage drive. The two-winding model has finite positive self-inductances, signed coupling in [−1, 1] and lossless linear windings. A separate ideal transformer accepts a finite, nonzero signed secondary-to-primary turns ratio. Both models support a resistive load or open circuit. Winding resistance, core loss, saturation, stray capacitance and general AC netlist stamps are outside this model.

## Run a sweep

The following examples use a 47 Ω load. The coupled model takes primary and secondary inductance in henries, followed by the signed coupling. The ideal model takes a signed voltage ratio. A negative ratio or coupling reverses the secondary voltage reference.

```sh
cargo run --release --locked -p electra-ac --bin electra-transformer -- coupled 1 100000000 101 0.001 0.004 0.97 47
cargo run --release --locked -p electra-ac --bin electra-transformer -- ideal 1 100000000 101 2 47
```

The CSV reports frequency in Hz, real and imaginary voltage gain, input port state, and real and imaginary input impedance in ohms. The ideal example has gain 2 and input impedance 11.75 Ω at every frequency. Use `open` for a disconnected secondary and `0` for a short. A perfectly coupled or ideal transformer with its secondary shorted rejects nonzero primary voltage drive. An open ideal input has blank impedance columns. The input of an open-secondary coupled pair draws magnetizing current. Both point APIs require a positive frequency. A DC bias requires a separate operating-point model.

## Winding equations and load convention

Voltages are measured at the indicated terminals relative to their returns. Currents enter those terminals, so the secondary equation for a resistor across the winding is V₂ = −RI₂. With exp(+jωt) phasors, ω = 2πf and signed mutual inductance M = k√(LₚLₛ), the winding law is

```math
\begin{bmatrix}V_1\\V_2\end{bmatrix}
= j\omega\begin{bmatrix}L_p&M\\M&L_s\end{bmatrix}
\begin{bmatrix}I_1\\I_2\end{bmatrix}.
```

Positive coupling corresponds to measuring both voltages at their dots. The magnitude bound on k enforces a positive semidefinite inductance matrix. [MIT 6.622, Magnetics Part 2](https://ocw.mit.edu/courses/6-622-power-electronics-spring-2023/mit6_622_s23_lec102.pdf)

For a positive load, write Xₚ = ωLₚ, Xₛ = ωLₛ and δ = (1 − |k|)(1 + |k|). Eliminating the branch currents gives

```math
H = \frac{V_2}{V_1}
= k\sqrt{L_s/L_p}\,\frac{R}{R+j\delta X_s},
\qquad
Z_{\mathrm{in}} = jX_p\delta + jX_p k^2\frac{R}{R+jX_s}.
```

The implementation specializes the same load calculation for dimensionless gain and ohms. It normalizes each division by its larger resistance or reactance and evaluates the leakage fraction in factored form. This avoids subtracting nearly equal squared inductances and preserves small reflected resistances when an intermediate ratio underflows. Open, short and uncoupled cases are prepared with their reduced equations. Unrepresentable required coefficients, reactances or entire nonzero responses return a numerical-range error. Individual quadratures follow binary64 rounding.

At |k| = 1 the loaded voltage ratio is exactly the signed turns ratio n = sign(k)√(Lₛ/Lₚ) in the circuit law. The input still sees the reflected resistance R/n² in parallel with jXₚ. For example, a reflected 10 Ω load in parallel with a magnetizing reactance of j10 Ω gives 5 + j5 Ω. Approaching the ideal *loaded* transformer requires both R/Xₛ and δXₛ/R to approach zero. An imperfectly coupled short instead presents jXₚδ, with zero secondary voltage.

## Qualification

The independent 70-digit decimal calculation applies a 1 V primary drive, assembles four real winding-current equations, pivots the matrix, and reads V₂ and 1/I₁. The comparison exercises three winding pairs, six signed coupling values, five terminations and ascending and descending sweeps from 1 Hz to 100 MHz. It covers 1,512 coupled samples and a further 144 ideal samples. The maximum measured relative error among compared nonzero quadratures was 5.38e−16 against a 1e−12 limit. A separate near-zero test handles residual decimal cancellation. The tests also check power balance, both polarities of the singular short, and component ratios beyond the range of direct squaring.

The ideal-limit regression increases Xₛ/R from 10 to 10,000 while reducing δ so that δXₛ/R falls from 0.1 to 0.0001. At each step it bounds the complex relative errors in both voltage gain and primary impedance against the ideal result. At the last step both are within 0.021%.

On macOS arm64 using Rust 1.95.0 the prepared coupled model occupied 56 bytes. A counting allocator observed zero allocations and reallocations during 10,001- and 100,001-point frequency sweeps. Release timings below are medians of 21 alternating runs after warm-up, using the 1 mH / 4 mH, k = 0.97, 47 Ω example above. They include grid iteration and accumulation of a checksum and exclude CSV formatting. Both modes produced identical checksums.

| Samples | Prepare at each frequency | Reuse the prepared model |
| ---: | ---: | ---: |
| 10,001 | 0.156 ms | 0.113 ms |
| 100,001 | 1.573 ms | 1.134 ms |

```sh
cargo test --workspace --all-features --locked
cargo build --release --locked -p electra-ac --bin electra-transformer
python3 crates/electra-ac/scripts/verify-transformer.py target/release/electra-transformer
cargo run --release --locked -p electra-ac --example profile-transformer -- prepared 100001
cargo run --release --locked -p electra-ac --example profile-transformer -- rebuild 100001
```
