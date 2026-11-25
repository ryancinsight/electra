# Thermal noise of an RC ladder

Electra computes the one-sided output-voltage power spectral density (PSD) of a passive RC ladder. Each series resistor and each shunt conductance has an independent temperature in kelvin. Capacitors are ideal and the input voltage source is noiseless. The resistance and conductance are supplied at the operating temperatures. The model uses classical Johnson–Nyquist noise, appropriate when the photon energy is small compared with the thermal energy. [NIST Johnson noise thermometry](https://nvlpubs.nist.gov/nistpubs/jres/122/jres.122.046.pdf)

## Run a sweep

The example has a 1 kΩ series resistor at 300 K and a shunt capacitance chosen for a 1 kHz pole. The five stage inputs are series resistance in Ω, shunt capacitance in F, shunt conductance in S, series temperature in K and shunt temperature in K. Append groups of five in input-to-output order for a longer ladder.

```sh
cargo run --release --locked -p electra-ac --bin electra-rc -- --noise 100 10000 3 1000 1.5915494309189535e-7 0 300 0
```

The CSV columns are `frequency_Hz` and `output_voltage_psd_V2_per_Hz`. The unloaded resistor alone gives 1.6567788 × 10⁻¹⁷ V²/Hz. At the pole the capacitor reduces this to 8.283894 × 10⁻¹⁸ V²/Hz. Integrate the PSD over a positive-frequency band to obtain mean-square volts. Its square root is an RMS voltage. The public point API accepts zero hertz as the right-hand limit. A source at zero kelvin contributes zero in the classical model.

## Circuit law

For each section let Zs be the upstream Thévenin impedance plus the series resistance R. With shunt admittance Y = G + j2πfC, the stage voltage transfer is h = 1/(1 + ZsY) and the loaded impedance is Z = Zs h. The same section implementation supplies the signal and noise calculations.

The series voltage source has density 4kᵦTᵣR in V²/Hz. The shunt current source has density 4kᵦTɢG in A²/Hz. Starting with zero upstream noise, one pass updates output voltage density by

```math
S_\mathrm{out} = (S_\mathrm{upstream} + 4 k_B T_R R) |h|^2 + 4 k_B T_G G |Z|^2.
```

The terms add in power because the sources are independent. Aequitas provides the Boltzmann constant and derives the spectral dimensions from its existing electrical quantities and frequency. The solver prepares the two densities once per stage and evaluates voltage and current transfers with the same statically specialized function. Weighting the complex components before squaring preserves representable powers when the transfer magnitude squared alone would overflow or underflow. If a positive prepared source or an entire positive stage result falls outside the representable range, the API reports an error.

The Iris adapter borrows the frequency and PSD slices. Its regression reads the 1 kHz coordinate and density directly from the solver samples. The headless trace is in V²/Hz. Amplitude-density plots, dB references and native rendering remain separate roadmap items.

## Verification and performance

The independent 70-digit decimal reference assembles the nodal admittance matrix with the input grounded, injects a 1 A test current at the output, and solves for all node voltages. By reciprocity, the voltage drop across each resistor gives its transfer from an equivalent noise current to the output. The oracle sums each temperature-weighted dissipation. It covers one to eight stages, five temperature distributions (equilibrium, mixed, series only, shunt only, all cold), and both sweep directions from 1 Hz to 10 MHz. All 720 CLI samples passed a 1e−12 relative tolerance for nonzero values and exact equality for zero. Maximum relative error was 8.23e−16. Additional tests cover the DC limit, the analytical pole, agreement with 4kᵦT Re(Z) in equilibrium, invalid inputs, and long chains in which local noise survives underflow of the end-to-end signal gain.

On macOS arm64 with Rust 1.95.0, a prepared stage occupied 40 bytes. The solver borrowed the stage array and used constant working storage per frequency. A counting allocator observed zero allocations and reallocations during each 10,001-point noise sweep. The table shows release medians of 15 interleaved runs after warm-up from 10 Hz to 10 kHz. Each stage used 75 Ω, 330 pF and 1 µS, with series and shunt temperatures of 310 K and 290 K. The signal comparison used the same pre-change profiling program on both revisions. Timing excludes model construction, CSV and display.

| Stages | Signal before | Signal with shared section | Thermal noise |
| ---: | ---: | ---: | ---: |
| 1 | 0.119 ms | 0.117 ms | 0.123 ms |
| 16 | 1.390 ms | 1.408 ms | 1.599 ms |
| 128 | 12.601 ms | 12.679 ms | 13.409 ms |

```sh
cargo test --workspace --all-features --locked
cargo test --locked --manifest-path integrations/iris-ac/Cargo.toml
cargo build --release --locked -p electra-ac --bin electra-rc
python3 crates/electra-ac/scripts/verify-noise.py target/release/electra-rc
cargo run --release --locked -p electra-ac --example profile -- 128 10001 noise
```
