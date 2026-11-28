# Infrared observations on two etched PCB heaters

The Hannover measurements provide a cross-check of how apparent temperature rise scales with measured electrical power for two etched trace layouts. Each coupon has a 100 × 100 × 1.6 mm FR4 board, a 90 × 90 mm heater on the underside and a dot grid viewed from above. Peter Shobowale, Lorenzo Ledesma and Markus Kästner supply the measurements and KiCad designs under CC BY-NC-SA 4.0. [Dataset and design files](https://doi.org/10.25835/0pua6j2o)

## Reproduce the held-out prediction

Use `Data.zip` (SHA-256 `fd6045afbc8ad98263603a5f3a623b9ef3404fce31bd5e12cfd0fc6e1fec0575`). In each run, match `power_data.csv` timestamps to `images/avg_frame_YYYYMMDD_HHMMSS.tiff` at exact seconds. The table identifies the three frames used. Decode unsigned 16-bit pixels as hundredths of a degree Celsius and compute power from the product of the recorded voltage and current.

| Run under `data/FR4/` | Date | Zero-input frame | Calibration frame and power | Held-out frame and power |
| --- | --- | --- | --- | --- |
| `Hilbert/temperature/0202/02` | 2026-04-01 | 17:02:19 | 17:22:08 · 4.99 V × 0.392 A | 17:41:58 · 9.99 V × 0.723 A |
| `Meander/temperature/0201` | 2026-03-10 | 17:04:55 | 17:24:44 · 4.99 V × 0.337 A | 17:44:32 · 9.99 V × 0.628 A |

At each pixel, estimate the local apparent temperature rise per watt from the zero-input and calibration frames. Predict the held-out rise as `(T5 − T0) × P10/P5`. Compare it with `T10 − T0` and take the root mean square (RMS) of prediction minus observation over all 1,024 × 768 pixels, including the background. The absolute-temperature maximum uses the maximum of `T0 + predicted_rise`. The maximum of a rise map can occur at a different pixel from the absolute-temperature maximum.

| Layout | Held-out input | Apparent maximum, observed / predicted | Maximum rise, observed / predicted | Full-frame RMS error |
| --- | ---: | ---: | ---: | ---: |
| Hilbert | 7.223 W | 51.52 / 54.00 °C | 31.34 / 33.90 K | 1.17 K |
| Meander | 6.274 W | 50.62 / 51.82 °C | 27.18 / 28.43 K | 0.50 K |

Three fixed camera pixels give the following observed / predicted rises in kelvin. Pixel coordinates are zero-based from the top-left corner.

| Layout | (256, 384) | (512, 384) | (768, 384) |
| --- | ---: | ---: | ---: |
| Hilbert | 30.46 / 31.72 | 30.95 / 32.42 | 29.87 / 30.28 |
| Meander | 26.33 / 26.19 | 26.46 / 25.82 | 25.14 / 25.78 |

## Limits of the comparison

These residuals test a linear power extrapolation between images of the same coupon at its own drive conditions. The coupons received different powers. The unpowered image measures apparent surface temperature. Both an air-temperature record and a calibrated mapping from image pixels to board coordinates are needed for a comparison against the physical mesh. The TIFF increment of 0.01 °C describes encoding resolution. The measurements reviewed here supply no uncertainty interval for the observed temperatures that could serve as a pass threshold. Physical validation still requires documented ambient, registration and measurement uncertainty for each coupon, together with the thermal boundary and material inputs used in the simulation. [Heater exit criterion](../checklist.md#v08)

## Independent simulation on the published layouts

[FiPy 4.0.3](https://github.com/usnistgov/fipy/tree/4.0.3) independently discretizes the steady sheet heat equation on a 3 × 3 cell-centered grid. FiPy uses its own diffusion and implicit sink terms with a SciPy direct LU solve. Electra uses a passive conductance network and an iterative nodal solve. Both apply zero normal flux on the four edges. [FiPy diffusion equations](https://pages.nist.gov/fipy/en/stable/FAQ.html) · [Boundary conditions](https://pages.nist.gov/fipy/en/stable/generated/examples.diffusion.mesh1D.html)

The input heat in each cell is proportional to the published trace centerline length inside it, assuming uniform current and trace cross-section. The design archive hash and segment counts appear in each fixture. The common 3 × 3 grid preserves the two layouts' distinct power distributions and permits comparison of every cell. It resolves temperatures averaged over cells 33⅓ mm across. The controlled model uses 20 °C ambient, a 1.6 mm substrate with effective conductivity 0.30 W/(m·K) and uniform heat rejection of 10 W/(m²·K). These are benchmark boundary and material assumptions. They do not represent measured air temperature or calibrated convection in the thermography runs.

| Layout | Applied power | FiPy peak rise | FiPy peak temperature | Peak center in board coordinates | Largest Electra residual against rounded FiPy reference |
| --- | ---: | ---: | ---: | --- | ---: |
| Hilbert | 7.22277 W | 87.097793 K | 107.097793 °C | (50, 50) mm | 0.481 µK |
| Meander | 6.27372 W | 74.312834 K | 94.312834 °C | (50, 50) mm | 0.310 µK |

The 1 µK comparison interval covers rounding the reference to six decimal places and the numerical solve. Each row of the passive discrete matrix has a sink conductance `g = h × cell area = 1/90 W/K`, so the maximum temperature error is at most the maximum cell power residual divided by `g` (apply the discrete maximum principle at the largest absolute error). The external check requires the FiPy residual to be at most 10⁻¹² W. For these positive fixtures Electra's zero-start relaxation stays between zero and `P/g`. The absolute sum of nodal flows including the source is bounded by `(2 + 4kt/g)P < 18 W` because `P < 8 W` and each lateral branch has conductance `kt = 0.00048 W/K`. With a zero absolute power floor and a relative residual limit of 10⁻¹⁰, Electra bounds its nodal residual by `18 W × 10⁻¹⁰`, giving less than 0.17 µK for these coupons including the floating-point residual allowance. The decimal reference adds at most 0.5 µK. This interval tests agreement of the two solvers on the declared mesh. The simulated peaks under these assumed boundary conditions are substantially higher than the observed infrared peaks.

From the repository root, with FiPy 4.0.3 available, the first command checks every committed reference against an independent solve. The two Rust replays compare Electra with all nine cell temperatures and the peak for their respective layouts.

```sh
python scripts/verify-heater-reference.py
cargo run --locked -p electra-board-field --example heater_coupon -- crates/electra-board-field/examples/heater_coupon/fixtures/hannover_hilbert_fipy.txt
cargo run --locked -p electra-board-field --example heater_coupon -- crates/electra-board-field/examples/heater_coupon/fixtures/hannover_meander_fipy.txt
```
