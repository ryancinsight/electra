# Heater coupon replay

`heater_coupon` replays a rectangular heater coupon against an external measurement or independent simulation; it does not supply reference data. Required evidence comprises physical stack-up, geometry, input power, ambient, heater registration, spatial point references and an independently recorded full-field peak. The executable reports simulated and reference rise, absolute simulated temperature, signed residual and comparison interval in SI units, and fails when a residual exceeds its interval. Equal finite volumes exchange heat through `sum(k t) dy/dx` or `sum(k t) dx/dy`, shed `h dx dy` to ambient and have reflecting exterior edges. `electra_network::solve` computes a coherent-kelvin potential converted to Aequitas `TemperatureDifference`; inputs also use typed `Power` and `ThermodynamicTemperature`. The thin-sheet approximation assumes one through-thickness temperature and uniform air conductance; it omits radiation, contact resistance, convection variation, subcell copper detail, layer gradients and temperature-dependent properties.

## Reference file

The bounded UTF-8 input is line-oriented. Blank lines and text after `#` are ignored. Records have these exact forms:

```text
format electra-heater-coupon-v1
coupon <identifier>
reference <measurement|simulation> <source-id>
geometry_mm <width> <height> <rows> <columns>
ambient_c <degrees-Celsius>
input_power_w <watts>
air_conductance_w_m2k <watts-per-square-metre-kelvin>
layer_mm_w_mk <layer-id> <thickness-mm> <conductivity-watt-per-metre-kelvin>
heater <zero-based-row> <zero-based-column> <power-fraction>
sensor_c <reference-id> <x-mm> <y-mm> <degrees-Celsius> <interval-K>
peak_c <x-mm> <y-mm> <degrees-Celsius> <interval-K>
```

At least one layer, heater and point reference and exactly one peak are required. Repeat layer, heater and sensor records. Origin is lower-left; rows advance in `y`, columns in `x`, and samples are bilinear between cell centres with the nearest cell extended to a reflecting edge. IDs and heater cells are unique; finite fractions in `(0, 1]` sum to one. The full replay rejects invalid geometry, material properties, source locations and reference coordinates. Limits are 4,096 cells, 1,024 references and 1,048,576 input bytes. Run `cargo run -p electra-board-field --example heater_coupon -- reference-coupon.txt`.

For `measurement`, intervals are externally recorded measurement uncertainties with calibration and acquisition records. For `simulation`, intervals are predeclared bounds derived from the independent solver's discretization, convergence and reference uncertainty; they are not instrument uncertainty and are never fitted from Electra residuals. Both the temperature sampled at the declared peak coordinate and the simulated global maximum must match the peak interval; this rejects a swapped coordinate without inventing a spatial tolerance, while a same-temperature footprint or plateau passes. Synthetic unit-conductance checks cover uniform, centre `[1/4, 1/2, 1/4] K`, edge `[5/8, 1/4, 1/8] K`, midpoint interpolation, zero load and rejection; they do not qualify a coupon. [Heater observations](heater-observations.md) record the published geometry and independent study. The two independent simulation fixtures replay the published layouts using controlled thermal boundaries. The observations document the numerical agreement and the remaining experimental limitations.

Each comparison includes uncertainty from the iterative solve. An interval that straddles the acceptance threshold is reported as unresolved. The rectangular mesh has a positive, uniform sink conductance on every cell. Applying the discrete maximum principle bounds the largest cell temperature error in kelvin by the largest nodal power imbalance in watts divided by that conductance in watts per kelvin. Electra pads the computed imbalance for floating-point evaluation before dividing. This bound applies to the discrete system assembled from the represented conductances and powers. Boundary-condition uncertainty and physical model error require separate evidence.
