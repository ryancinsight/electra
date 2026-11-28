# Uniform transmission-line qualification

The radial finite-volume solve holds ideal concentric conductors at fixed potentials. It conserves flux through each annular shell, accumulating the potential drop with midpoint face area. In a homogeneous dielectric the magnetic solution uses the same geometric integral. Both the extraction and frequency response need constant auxiliary storage.

The reference has radii 0.25 and 1 mm, relative permittivity 2.1, vacuum permittivity 8.8541878128 pF/m and permeability 4π × 10⁻⁷ H/m. Compare 8, 16 and 32 uniform shells to C′ = 2πε / ln(b/a) and L′ = μ ln(b/a) / 2π. The gate allows 2% absolute relative error and 1% change on the final refinement. This geometry qualifies axisymmetric external TEM fields with homogeneous fill and ideal conductors. Planar microstrip, dispersion and internal inductance require separate qualification.

A one-metre 250 nH / 100 pF lossless line has a 50 Ω characteristic impedance and 5 ns one-way transit. At 1–200 MHz in 1 MHz increments the tests de-embed matched/open/short loads, recover delay from the transmission phase slope and check the largest scattering singular value. A second line with 3 Ω series loss and 1 mS shunt loss has independent ABCD-to-S checks in both directions at 75 Ω reference impedance.

Run `cargo test --locked -p electra-ac --test interconnect`.
