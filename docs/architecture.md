# Electrical and network ownership

## Grounding in existing Atlas code

| Inspected implementation | Reusable behavior | Behavior owned by the consumer |
| --- | --- | --- |
| [CFDrs matrix assembly](https://github.com/ryancinsight/CFDrs/blob/40021147e365520bd0e763f258638540f19ba1ff/crates/cfd-1d/src/solver/core/matrix_assembly.rs) | A passive edge contributes +g on both diagonals and −g off diagonal. Prescribing an endpoint moves g times its potential to the free row’s right-hand side. The preview delegates this operation to Electra. | Geometry and viscosity determine hydraulic resistance. CFDrs owns pressure/flow types, boundary classification, nonlinear coefficient updates, sparse storage and solve selection. |
| [kwavers-driver IR drop](https://github.com/ryancinsight/kwavers/blob/7d6c4b89d5acbe223391d08abba2ec144382c7bb/crates/kwavers-driver/src/physics/thermal/ir_drop.rs) | The board path performs ordered Gauss–Seidel sweeps over conductance-weighted neighbours. The preview uses the same Electra sweep as the DC solver. | Track geometry, copper thickness, selection of supply and leaves, equal load distribution, fixed sweep count and maximum-drop convention. |
| [kwavers-driver electronics](https://github.com/ryancinsight/kwavers/blob/7d6c4b89d5acbe223391d08abba2ec144382c7bb/crates/kwavers-driver/src/physics/pdn/impedance.rs) and [public KiCad consumer](https://github.com/ryancinsight/kwavers/blob/7d6c4b89d5acbe223391d08abba2ec144382c7bb/scripts/tests/driver_consumer/tests/kicad.rs) | Candidate owners for PDN impedance, board structures, placement/routing, DRC and KiCad readers/writers. The existing consumer checks routed pairs and schematic labels. | Transducer drive protocols, array configuration, acoustic loading, experiment presets and application-specific limits stay in kwavers. |

For a two-terminal impedance analogy, voltage corresponds to pressure and current to volumetric flow. Resistance corresponds to pressure drop per unit flow. Compliance and inertance are candidates for future C/L analogues. The CFD adapter passes dimensional coefficients and pressure to the shared stamp. It extracts magnitudes only when writing the consumer’s scalar sparse matrix and RHS. Fluxes and tolerances retain their domain units.

## Dependency direction

The numerical kernel accepts indexed endpoints, coefficients and borrowed iterators. CFDrs can stamp directly into Leto COO and its own RHS buffer. The driver can borrow its CSR neighbour runs and choose its own sweep count. The electrical API applies circuit conventions and owns the neutral/reference node. Topology and electrical element behavior have separate owners so downstream consumers can depend on the small network crate without importing an EDA application.

Keep one circuit connectivity model for CLI, editors, KiCad interchange and simulation. Bind device models explicitly to symbol pins and physical pads. Allocate only at topology compilation, assembly or result boundaries where feasible. DC transfer analysis now reuses its topology, sparse row storage and solver workspace across points. Group devices by concrete model for stamped hot loops. Resolve a solver/backend once per analysis and measure specialization time, binary size, allocations and throughput before adding another specialization.

| Planned capability | Atlas owner or intended integration |
| --- | --- |
| Dimensional public quantities, numeric scalar and complex arithmetic | Aequitas is active in both 0.1 crates for typed physical quantities. Eunomia supplies its scalar foundation. Qualify each additional model quantity before use. |
| Dense and sparse arrays, direct and iterative linear solves | Leto/Leto Ops and Athena. A grounded resistor block is positive definite under the usual passive assumptions. Modified nodal analysis with ideal voltage sources can be indefinite or nonsymmetric; qualify pivoting, scaling and diagnostics on those matrices. |
| Time integration, spectral analysis, parallel parameter studies | Evaluate Horae for stiff DAEs and event handling, Apollo for spectra, Moirai for work scheduling. Preserve a deterministic serial solve. |
| Geometry, material coefficients, multiphysics exchange, optional device compute | Gaia, Proteus, Harmonia and Hephaestus. Electra owns electrical laws and boundary conditions. Add a GPU route only with host parity and a measured benefit. |
| Model/results storage and native editor | Consus for structured storage where required, Metis for native interaction, and Iris for plots, geometry and field visualization. Keep the headless design and simulation APIs callable without a display. |

## Prepared DC analysis

`electra-network::PreparedNetwork` borrows passive branches. It classifies fixed nodes, checks that each component has a reference, and packs free-node neighbour runs in input order. The temporary row offsets also support the anchoring check. The solver keeps the resulting row lengths, diagonals and fixed-neighbour terms, then releases the compilation scratch. The generic relaxation kernel consumes the runs by iterator. Each solve rebuilds its right-hand side and initial guess in place and checks physical branch fluxes against the typed tolerances. The one-shot `solve` entry point moves the same workspace’s solution into an owned result.

`electra-circuit` resolves a sweep source once, builds the contributions of all other current sources once and updates the swept excitation between points. A floating-voltage sweep rebuilds source-tree offsets and the corresponding reduced injections in reusable buffers. Omitting the swept source before summing keeps a large nominal excitation from erasing small independent loads. DC analysis projects both nominal and swept solutions into the same named probe representation. A borrowed readout limits result storage to one point. A network workspace can retry an error. A circuit sweep returns its error once and terminates, allowing clients to retain the valid prefix.

Independent voltage-source connections form a forest when individual currents are unique. The constraint compiler groups each tree into a supernode and preserves the original node and part indices. The `analysis/dc` session uses the same monomorphized `electra-network` relaxation kernel for both original and reduced resistor graphs. On the reduced path, a statically dispatched acceptance callback reconstructs physical voltages and source currents and can request additional iterations. Ground-referenced source decks retain the boundary-elimination path. [Equations and qualification](floating-sources.md)

The MNA path uses the same circuit, named probes, excitation grid and compensated current injection assembly. Electra owns physical scaling, sparse terminal stamps and physical acceptance. Athena Core owns GMRES, workspace layout contracts, Arnoldi iteration and termination. Its fixed-width host specialization has no dynamic dispatch in the operator or vector kernels. [MNA equations, qualification and cost](modified-nodal-analysis.md)

## Hierarchical netlist compilation

File loading records source spans. Declaration collection establishes the model namespace before linking each definition once to integer node, parameter and model slots. An iterative postorder traversal checks recursion, propagates used pins and counts expanded primitives. The expander reserves physical part and resistor storage, reuses node and parameter scratch across siblings and emits directly into the same circuit builder as the flat reader. The DC solver receives concrete resistors, independent sources and typed controlled-source laws. Named current controls resolve to voltage-branch indices during elaboration. A single stamp traversal specializes for matrix products and sparse rank checking. [Controlled-source implementation and qualification](controlled-sources.md) Instance lookup, parameter binding and file access finish before the solve. Diagnostic paths are resolved back to source cards on failure. [Grammar, validation and measurements](hierarchical-netlists.md)

## Atlas-only dependency rule

Electra takes direct dependencies only on Atlas packages. Both crates use Aequitas 0.5.0 with default features disabled. The locked graph contains two Electra packages, four Atlas provider packages (Aequitas, Athena Core, Eunomia and Eunomia Derive), and nine inherited third-party package versions. Typenum provides dimension arithmetic. Eunomia and its macro/build dependencies bring in bytemuck, bytemuck_derive, libm, proc-macro2, quote, two versions of syn and unicode-ident. The source of these dependencies is the Atlas providers. The CLI, solver and tooling add no independent external engine or library.

The dependency audit rejects third-party direct declarations (including dev/build and platform dependencies), unreviewed sources, and increases outside the reviewed inherited package versions. `python3 scripts/check-dependencies.py --strict-transitive` also rejects the current inherited closure. Keep that result open on the release checklist. Work toward reducing it in the owning Atlas providers; check dimension laws, packed scalar behavior and no-default-feature builds before removing a dependency. Adding a local imitation of Aequitas would introduce competing unit definitions.

The optional Leto direct adapter has its own workspace and lockfile. Electra supplies the normalized operator and physics checks. Leto owns pivoting and triangular solves. Run `python3 scripts/check-dependencies.py --leto-direct` to audit the adapter. [Qualification and resource limit](pivoted-direct.md)

KiCad and SPICE are interoperability and validation targets. Archive reference inputs, versioned external outputs where redistribution permits, and analytical derivations. A user must be able to install Electra, simulate a project, edit a board and generate its fabrication files using Atlas code. Reference executables can be used independently to generate comparison data. They are optional to the developer and absent from the product runtime.

## Migration order

1. Qualify the numerical stamps and sweep against pinned consumer tests. Obtain a resolvable Electra revision/package for each consumer and verify from a clean checkout.
2. Switch the CFDrs assembly and driver sweep call sites behind their existing public APIs. Preserve CFD units, source signs, convergence policy and the driver’s fixed-budget result. Compare runtime and allocations on real network sizes.
3. Move electrical models and KiCad/board functionality one vertical slice at a time. Electra owns each migrated implementation. Maintain the driver’s re-exports or delegating adapters until its consumers have migrated. Keep independent validation fixtures and obsolete implementation removal in the same change.
4. Register Electra in the Atlas member inventory and release/evidence boards after its hosted revision is accessible to intended consumers. Pin member and consumer revisions together. Leave source incorporation pending until each consumer’s full release gates pass.

Design references: [KiCad file formats](https://dev-docs.kicad.org/en/file-formats/) and [KiCad simulation workflow](https://www.kicad.org/discover/spice/).

## Presentation ownership and typed data

| Owner | Existing seam | Extension and qualification |
| --- | --- | --- |
| Metis | Native application, commands, input events and presentation surfaces. [Provider baseline](https://github.com/ryancinsight/metis/blob/9a126091fdcabf908de9eb5e6d6e26caa5b7f885/README.md) | Extend Metis for reusable canvas gestures, keyboard navigation, hit testing, accessibility, frame presentation and readback where its current API is insufficient. Test pointer and keyboard edits, undo and cross-probing through the native event path. |
| Iris | Borrowed paired-series and scalar-field views, axis metadata, color mapping and a static render-backend seam. [Provider baseline](https://github.com/ryancinsight/iris/blob/e257ce9996a3c6d0ffb1dbbd1fac4c8fd3e8fe1e/README.md) | Extend Iris for unit-aware axes, log scales, multiple traces, field overlays and reusable 2D/3D view primitives as each feature lands. Test typed samples, numeric cursor readback, missing values, clipping and legend bounds before pixel comparisons. |
| Electra | Electrical model, analysis settings, topology and stable net/pin/probe IDs | Map circuit or board semantics into Metis commands and Iris views. Schematic and PCB editing, waveform inspection, 3D preview, DRC overlays and fabrication preview all use these providers. A new shared capability lands in its owning Atlas repository with tests and a pinned Electra consumer fixture. |
| Aequitas | Dimensioned SI quantities and unit conversion | Use its quantities for every physical value from model input through solver, probe, display and export. Carry dimensions through stamps, residuals, absolute tolerances and relative tolerances. Convert magnitude only at text/format, finite-value checks or scalar-only provider boundaries. IDs, iteration counts, event states and pixel indices keep their discrete types. |

The 0.1 electrical and hydraulic operating points use the same dimension-checked equations. Electrical conductance times voltage produces current; hydraulic conductance times pressure produces volumetric flow. For future geometry, time, frequency, complex impedance, thermal, field and spectral quantities, add missing dimensions or units to Aequitas first. Axis labels alone cannot recover lost dimensions. Physical design coordinates stay distinct from screen pixels; a transform and its inverse have round-trip tests. Keep headless analysis usable without Metis or Iris.
