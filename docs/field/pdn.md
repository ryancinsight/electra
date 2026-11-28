# Via and decoupling comparison

The qualification fixture is a passive analytical coupon, not a measured board. It fixes a 1.6 mm homogeneous board, 20 µm via-wall plating, copper resistivity of 1.724 × 10⁻⁸ Ω m, a two-capacitor budget, six vias (one feed pair and one pair per capacitor), an 80 mΩ target, and the 13-point grid from 10 kHz through 100 MHz declared in [`pdn.rs`](../../crates/electra-board-field/tests/pdn.rs). Every candidate uses the same grid and budget.

The fixed-via control uses 0.20 mm drills at 1.00 mm center pitch. The joint search also considers 0.20 mm drills at 0.60 mm pitch and 0.25 mm drills at 0.45 mm pitch. At every geometry the search enumerates the same three two-part bills of materials: 10 µF/20 mΩ/1.0 nH with 100 nF/50 mΩ/0.50 nH; 4.7 µF/18 mΩ/0.70 nH with 220 nF/35 mΩ/0.35 nH; and 2.2 µF/15 mΩ/0.60 nH with 470 nF/25 mΩ/0.40 nH. Thus capacitor selection is optimized in the fixed-via control as well as in each joint alternative.

For via length `h`, drill diameter `d`, thin plating `t`, center spacing `D`, and resistivity `ρ`, the power/return pair contributes

`Rpair = 2ρh/(πdt)` and `Lpair = μ0 h/π acosh(D/d)`.

The first expression uses the thin-wall area approximation `πdt`, valid when plating thickness is small relative to drill diameter. In the second expression, the cited round-wire relation uses conductor radius `a` and `D/(2a)`; this fixture equates `2a` with the plated-hole drill diameter. NASA's *2-Wire Transmission Line Parameters*, pages 45–46, gives the quasi-static relation and defines `D` as center spacing and `a` as radius ([NASA Technical Reports Server](https://ntrs.nasa.gov/citations/20230007845)).

Each connection pair adds its resistance and inductance to the capacitor's ESR and ESL. The existing complex [`CapacitorBank`](../../crates/electra-ac/src/pdn/mod.rs) evaluates those branches in parallel; the feed pair is then added in series. The test independently forms each branch reciprocal with rectangular arithmetic and checks both real and imaginary impedance at all 117 candidate-frequency points.

This is an analytic comparison of ideal straight, round, nonmagnetic conductors. It omits plane spreading impedance, pad and antipad fields, mutual coupling, dielectric dispersion, skin/proximity effects, mounting-pad inductance beyond the declared component ESL, fabrication tolerance, and temperature change. The sampled minimum is reported as a grid-resonance estimate, not an extracted continuous resonance. A manufactured stack-up needs field extraction and calibrated measurements before the result supports hardware limits.

## Results

The minimax score is the largest sampled `|Z|/80 mΩ`; lower is better. All nine candidates exceed the target on this deliberately broad grid. The joint moderate geometry with the bulk/local BOM wins. The fixed-via control independently chooses the same BOM from its three options, so a BOM-only change does not explain the improvement.

| Rank | Candidate | Worst ratio | Largest excess (Ω) | Worst frequency | Grid minimum (Ω) | Minimum frequency | Budget |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | moderate / bulk-local | 19.6981 | 1.49585 | 10 kHz | 0.0289863 | 1 MHz | 2 parts, 6 vias |
| 2 | compact / bulk-local | 19.6983 | 1.49586 | 10 kHz | 0.0268677 | 1 MHz | 2 parts, 6 vias |
| 3 | fixed / bulk-local | 20.0612 | 1.52489 | 100 MHz | 0.0299591 | 1 MHz | 2 parts, 6 vias |
| 4 | fixed / wideband | 40.4343 | 3.15474 | 10 kHz | 0.0278275 | 1 MHz | 2 parts, 6 vias |
| 5 | moderate / wideband | 40.4348 | 3.15478 | 10 kHz | 0.0296130 | 1 MHz | 2 parts, 6 vias |
| 6 | compact / wideband | 40.4352 | 3.15481 | 10 kHz | 0.0268050 | 2 MHz | 2 parts, 6 vias |
| 7 | fixed / midband | 74.5088 | 5.88071 | 10 kHz | 0.0222345 | 2 MHz | 2 parts, 6 vias |
| 8 | moderate / midband | 74.5093 | 5.88074 | 10 kHz | 0.0203744 | 2 MHz | 2 parts, 6 vias |
| 9 | compact / midband | 74.5097 | 5.88078 | 10 kHz | 0.0200755 | 2 MHz | 2 parts, 6 vias |

The selected joint trace and optimized fixed-via control are shown at every declared frequency. Values are impedance magnitude in Ω; the executable test also checks both quadratures.

| Frequency | Joint moderate / bulk-local | Fixed / bulk-local |
|---:|---:|---:|
| 10 kHz | 1.575847 | 1.575805 |
| 20 kHz | 0.788007 | 0.787923 |
| 50 kHz | 0.315438 | 0.315228 |
| 100 kHz | 0.158148 | 0.157733 |
| 200 kHz | 0.079999 | 0.079211 |
| 500 kHz | 0.035667 | 0.034442 |
| 1 MHz | 0.028986 | 0.029959 |
| 2 MHz | 0.044208 | 0.051151 |
| 5 MHz | 0.118795 | 0.146816 |
| 10 MHz | 0.113166 | 0.110801 |
| 20 MHz | 0.231159 | 0.296052 |
| 50 MHz | 0.634635 | 0.795369 |
| 100 MHz | 1.283712 | 1.604893 |

