# Electrical and network ownership

## Grounding in existing Atlas code

| Inspected implementation | Reusable behavior | Behavior owned by the consumer |
| --- | --- | --- |
| [CFDrs matrix assembly](https://github.com/ryancinsight/CFDrs/blob/40021147e365520bd0e763f258638540f19ba1ff/crates/cfd-1d/src/solver/core/matrix_assembly.rs) | A passive edge contributes +g on both diagonals and −g off diagonal. Prescribing an endpoint moves g times its potential to the free row’s right-hand side. The preview delegates this operation to Electra. | Geometry and viscosity determine hydraulic resistance. CFDrs owns pressure/flow types, boundary classification, nonlinear coefficient updates, sparse storage and solve selection. |
| [kwavers-driver IR drop](https://github.com/ryancinsight/kwavers/blob/7d6c4b89d5acbe223391d08abba2ec144382c7bb/crates/kwavers-driver/src/physics/thermal/ir_drop.rs) | The board path performs ordered Gauss–Seidel sweeps over conductance-weighted neighbours. The preview uses the same Electra sweep as the DC solver. | Track geometry, copper thickness, selection of supply and leaves, equal load distribution, fixed sweep count and maximum-drop convention. |
| [kwavers-driver electronics](https://github.com/ryancinsight/kwavers/blob/7d6c4b89d5acbe223391d08abba2ec144382c7bb/crates/kwavers-driver/src/physics/pdn/impedance.rs) and [public KiCad consumer](https://github.com/ryancinsight/kwavers/blob/7d6c4b89d5acbe223391d08abba2ec144382c7bb/scripts/tests/driver_consumer/tests/kicad.rs) | Candidate owners for PDN impedance, board structures, placement/routing, DRC and KiCad readers/writers. The existing consumer checks routed pairs and schematic labels. | Transducer drive protocols, array configuration, acoustic loading, experiment presets and application-specific limits stay in kwavers. |

For a two-terminal impedance analogy, voltage corresponds to pressure and current to volumetric flow. Resistance corresponds to pressure drop per unit flow. Compliance and inertance are candidates for future C/L analogues. The CFD adapter converts its typed coefficients at the numerical boundary. Fluxes and tolerances retain their domain units.

## Dependency direction

The numerical kernel accepts indexed endpoints, coefficients and borrowed iterators. CFDrs can stamp directly into Leto COO and its own RHS buffer. The driver can borrow its CSR neighbour runs and choose its own sweep count. The electrical API applies circuit conventions and owns the neutral/reference node. Topology and electrical element behavior have separate owners so downstream consumers can depend on the small network crate without importing an EDA application.

Keep one circuit connectivity model for CLI, editors, KiCad interchange and simulation. Bind device models explicitly to symbol pins and physical pads. Allocate only at topology compilation, assembly or result boundaries where feasible. Future analyses should reuse topology, symbolic sparsity and workspace across sweeps. Group devices by concrete model for stamped hot loops. Resolve a solver/backend once per analysis and measure specialization time, binary size, allocations and throughput before adding another specialization.

| Planned capability | Atlas owner or intended integration |
| --- | --- |
| Dimensional public quantities, numeric scalar and complex arithmetic | Aequitas and Eunomia. Qualify electrical quantities at the public boundary before expanding model APIs. |
| Dense and sparse arrays, direct and iterative linear solves | Leto/Leto Ops and Athena. A grounded resistor block is positive definite under the usual passive assumptions. Modified nodal analysis with ideal voltage sources can be indefinite or nonsymmetric; qualify pivoting, scaling and diagnostics on those matrices. |
| Time integration, spectral analysis, parallel parameter studies | Evaluate Horae for stiff DAEs and event handling, Apollo for spectra, Moirai for work scheduling. Preserve a deterministic serial solve. |
| Geometry, material coefficients, multiphysics exchange, optional device compute | Gaia, Proteus, Harmonia and Hephaestus. Electra owns electrical laws and boundary conditions. Add a GPU route only with host parity and a measured benefit. |
| Model/results storage and native editor | Consus where structured storage is required and Metis for native UI. Keep the headless design and simulation APIs callable without a display. |

## Atlas-only dependency rule

Electra product crates, runtime engines, build dependencies and test libraries must come from Atlas repositories or the Rust toolchain. The 0.1 dependency check enumerates the entire resolved Cargo graph, including workspace feature selection. It rejects every package outside this workspace. Introducing another Atlas provider requires recording its repository and reviewing its full resolved closure. If that brings in third-party packages, narrow features or remove the dependency in the provider before adding it. Do not vendor or rename an external dependency to pass the check.

KiCad and SPICE are interoperability and validation targets. Archive reference inputs, versioned external outputs where redistribution permits, and analytical derivations. A user must be able to install Electra, simulate a project, edit a board and generate its fabrication files using Atlas code. Reference executables can be used independently to generate comparison data. They are optional to the developer and absent from the product runtime.

## Migration order

1. Qualify the numerical stamps and sweep against pinned consumer tests. Obtain a resolvable Electra revision/package for each consumer and verify from a clean checkout.
2. Switch the CFDrs assembly and driver sweep call sites behind their existing public APIs. Preserve CFD units, source signs, convergence policy and the driver’s fixed-budget result. Compare runtime and allocations on real network sizes.
3. Move electrical models and KiCad/board functionality one vertical slice at a time. Electra owns each migrated implementation. Maintain the driver’s re-exports or delegating adapters until its consumers have migrated. Keep independent validation fixtures and obsolete implementation removal in the same change.
4. Register Electra in the Atlas member inventory and release/evidence boards after its hosted revision is accessible to intended consumers. Pin member and consumer revisions together. Leave source incorporation pending until each consumer’s full release gates pass.

Design references: [KiCad file formats](https://dev-docs.kicad.org/en/file-formats/) and [KiCad simulation workflow](https://www.kicad.org/discover/spice/).
