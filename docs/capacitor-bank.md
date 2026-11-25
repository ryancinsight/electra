# Capacitor-bank impedance

Electra evaluates a single port with any number of passive series RLC branches in parallel. Positive current enters the port from the source and returns through ground. The impedance is port voltage divided by that current, using phasors with time dependence exp(+jωt). Capacitance, ESR and ESL are constant values at the temperature of interest. Supply measured or rated values for that temperature. The model has no temperature coefficient, frequency-dependent dielectric loss, mutual coupling, or layout parasitics beyond the supplied ESL.

For branch k the impedance is Rₖ + j(ωLₖ − 1/(ωCₖ)), with ω = 2πf. Electra scales each rectangular reciprocal before squaring its components and uses compensated summation for the real and imaginary admittances. It inverts the total admittance once. A single branch returns its rectangular impedance directly. The typed API borrows the validated branch slice and uses constant workspace per frequency.

At DC every capacitor is open. An empty bank is also open. An ideal lossless branch with zero computed reactance shorts the entire bank, even in parallel with a lossy branch. Exactly cancelling admittances give an open port. There is no resonance tolerance or artificial damping. Near a pole the result is sensitive to component and frequency rounding. An arithmetic range error reports an unrepresentable intermediate or result.

## Inputs and output

`electra-pdn START_HZ STOP_HZ POINTS C_F ESR_OHM ESL_H ...` takes one triple per physical capacitor. Capacitance must be finite and positive. ESR and ESL must be finite and nonnegative. The CLI uses Electra's positive logarithmic frequency grid. The API also accepts a point at zero hertz. The CLI accepts numeric triples and diagnoses invalid or incomplete input. SPICE AC device and model cards require the future general AC netlist reader.

The CSV includes frequency in Hz, port state, real and imaginary impedance in Ω, and impedance magnitude in Ω. An open port has blank rectangular components and infinite magnitude. A short has zero in all three impedance columns.

## Verification

Run from the Electra workspace root, with a Kwavers clone containing the reference revision.

```sh
cargo test --locked -p electra-ac --test capacitor_bank
cargo build --locked --release -p electra-ac --bin electra-pdn
python3 crates/electra-ac/scripts/verify-pdn.py target/release/electra-pdn /path/to/kwavers
```

The verifier compiles the original Kwavers impedance function from Git blob `3a7bc6c1834d28ba698587c31cb665153c1f79bd`. It compares its magnitude with the Electra CLI. A second calculation compares both quadratures against the repeated two-branch identity ZₐZᵦ/(Zₐ + Zᵦ). The fixture uses the driver's 10 µF, 20 mΩ, 10 nH bulk branch and its 100 nF, 50 mΩ, 0.5 nH local branch. The three banks contain one local branch, bulk plus local, and bulk plus two local branches. Each sweeps 101 points from 100 Hz to 100 MHz in both directions. All 606 points met the 1e−12 relative tolerance. The maximum relative errors were 6.66e−16 for driver magnitude and 1.83e−15 for complex impedance. [Kwavers reference implementation](https://github.com/ryancinsight/kwavers/blob/daf3842e05e049f8f02a7fb31bb34f640e242e29/crates/kwavers-driver/src/physics/pdn/impedance.rs)

A separate series RLC fixture with R = 2 Ω, L = 100 µH and C = 1 µF locates the resonance at the reactance zero and estimates Q from the half-power bandwidth. The recovered values agree with 1/(2π√LC) ≈ 15.915 kHz and √(L/C)/R = 5 within 1e−12 relative error. The ideal-limit tests exercise series shorts and parallel poles separately. At exact zero series impedance the legacy driver skips the branch. Electra returns the physical short circuit.

On macOS arm64, an external counting allocator observed zero allocations and reallocations during each 10,001-point sweep for 1, 2 and 128 branches. The median times across nine release runs were 0.112 ms, 0.264 ms and 5.15 ms respectively. These timings cover grid iteration and magnitude evaluation and exclude component construction and CSV formatting.
