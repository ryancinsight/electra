# RC frequency response

The headless solver handles an ideal voltage source driving a ladder of series resistors. Each stage can have a capacitor and a conductance to ground. Results are the voltage gain from the source to the last node and the Thévenin impedance looking back into that node with the source shorted. The frequency domain uses exp(+jωt). The same amplitude convention applies to input and output. The API accepts DC as a point. Logarithmic grids require positive frequencies.

## Using the solver

Run the commands from the Electra workspace root.

```sh
cargo run --locked --release --manifest-path crates/electra-ac/Cargo.toml --bin electra-rc -- 100 10000 3 1000 1.5915494309189535e-7 0
```

The first three arguments are start frequency in Hz, stop frequency in Hz and total points, including both endpoints. Append an input-to-output triple for each stage in Ω, F and S. The example sets a 1 kHz pole. CSV columns carry frequency, real and imaginary gain, linear magnitude, principal phase in degrees and the real and imaginary output impedance in Ω. Values at 100 Hz, 1 kHz and 10 kHz are shown rounded below.

| Frequency | Complex voltage gain | Magnitude | Phase | Output impedance |
| ---: | --- | ---: | ---: | --- |
| 100 Hz | 0.990099 − j0.099010 | 0.995037 | −5.710593° | 990.099 − j99.010 Ω |
| 1 kHz | 0.5 − j0.5 | 0.707107 | −45.000000° | 500 − j500 Ω |
| 10 kHz | 0.009901 − j0.099010 | 0.099504 | −84.289407° | 9.901 − j99.010 Ω |

`Stage::new` validates physical parameters. `RcLadder::new` borrows the stages. `RcLadder::at` computes one checked frequency point. The optional `LogSweep` streams increasing or decreasing frequencies and returns a diagnostic if two requested samples would coincide in floating point. The last sample uses the exact supplied endpoint. Results use Aequitas quantities over the Eunomia complex scalar. A direct voltage gain is dimensionless, phase carries angle semantics, and output impedance is dimensioned. Model validation and numerical errors identify invalid inputs or the first unrepresentable stage.

## Circuit law and storage

Begin with gain H = 1 and output impedance Z = 0. For each new stage form S = Z + R, Y = G + j2πfC and D = 1 + SY. The new gain is H/D and the new output impedance is S/D. Propagating the impedance retains interstage loading. For two identical unloaded sections the gain at the individual section pole is −j/3. Treating the sections as buffered would yield −j/2.

The kernel makes one forward pass over borrowed stages per frequency. Stage storage is 24 bytes each on the tested target. The calculation retains a constant number of complex values, independent of node and frequency count. Caching the common reciprocal avoids two complex divisions per stage. At large divisor magnitudes the kernel scales both numerators before division so a representable impedance component survives. Nonfinite intermediates, an erased positive susceptance, or complete underflow of the gain or impedance return a range error.

## Verification

The 70-digit decimal reference assembles the real and imaginary nodal equations independently, pivots the resulting real block matrix, and solves for a normalized voltage drive and a separate 1 A output test injection. The CLI comparison covered 32 seeded models with 1–8 stages, both sweep directions and 13 frequencies per model. Across 416 samples all 832 complex comparisons passed. Maximum relative errors were 1.03e−15 for gain and 5.26e−16 for impedance. Maximum wrapped phase difference was 2.85e−14 degrees. Separate tests cover the analytical pole, both adjacent decades, DC loading, extreme impedance scale, invalid inputs, degenerate grids and compile-time unit errors.

A release-mode benchmark on Linux x86-64, Intel Xeon Platinum 8573C, Rust 1.95.0 streamed 10,001 frequencies from 10 Hz to 10 kHz. Each stage used 75 Ω, 330 pF and 1 µS. Times below are medians of 15 interleaved runs after warm-up. The comparison used the same implementation with divisor scaling at every stage. Timing excludes CSV and includes grid evaluation. A separate counting system allocator observed zero allocations and zero reallocations between grid construction and sweep completion at all three sizes.

| Stages | Always scale | Scale for large divisors |
| ---: | ---: | ---: |
| 1 | 0.620 ms | 0.584 ms |
| 16 | 4.783 ms | 3.754 ms |
| 128 | 38.793 ms | 27.138 ms |

```sh
cargo test --locked --manifest-path crates/electra-ac/Cargo.toml
cargo test --locked --manifest-path integrations/iris-ac/Cargo.toml
cargo build --release --locked --manifest-path crates/electra-ac/Cargo.toml --target-dir target --bin electra-rc
python3 crates/electra-ac/scripts/verify-ac.py target/release/electra-rc
cargo run --release --locked --manifest-path crates/electra-ac/Cargo.toml --example profile -- 128 10001
```

## Presentation and roadmap

`electra-iris-ac` wraps two Iris 0.1.4 `SeriesView` instances around borrowed frequency, dimensionless magnitude and angle slices. The selected index returns the actual solver coordinate and both ordinates. The cursor regression reads exactly 1,000 Hz from an explicitly requested 1,000 Hz sample. A compile-fail test rejects a voltage slice as a gain axis. This adapter prepares headless data for a Metis view.

RC ladders now propagate independent resistor sources and expose a borrowed Iris voltage PSD trace. [Thermal-noise model](thermal-noise.md)

General R/C netlist stamping, arbitrary connectivity, coupled devices, calibrated dB references and the native Metis Bode/PSD view remain open. A future frequency-domain matrix solve can use these loaded ladders and the independent nodal reference as regression fixtures.
