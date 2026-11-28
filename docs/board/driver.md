# Driver board extraction

The source baseline is [kwavers 34fa3f47](https://github.com/ryancinsight/kwavers/tree/34fa3f47ae07e28d433b10a22c746f2c982e042e/crates/kwavers-driver/src). The production migration merged in [kwavers #1177](https://github.com/ryancinsight/kwavers/pull/1177) at `4ec0d254`. Its lockfile pins the shared provider at `7a5d4c65`. [Consumer CI](https://github.com/ryancinsight/kwavers/actions/runs/46094394493). The driver suite passed 495 tests on the baseline and 495 tests with the extracted types delegated locally. The suite includes per-layer differential-pair length, pad entry, routing, and repeated-UUID cases.

| Electra module | Extracted driver responsibilities | Remaining driver consumers |
|---|---|---|
| `geom` | Integer nanometres, points, planar distances, polygon containment and hull | The driver's grid specification, mechanics, floorplan, router, physics and exporters use the public geometry facade. |
| `board` | Nets, layer IDs, pad connectivity, tracks, vias, zone boundaries and storage | `board::Board` specializes the shared storage to the driver grid. Routing, audit, fabrication, simulation and IO continue to use that alias. |
| `place` | Role classification, placement rotation and legal rotation policy | The driver owns footprint catalogues, the annealing schedule and its physics-guided placement objectives. |
| `route` | Midpoint and per-layer electrical trace length in original segment order | Pair auditing, coupled routing and the negotiated-congestion search remain in the driver. |
| `kicad` | Deterministic sequence UUID formatting and duplicate UUID audit | Driver board and schematic writers retain their emission order and generator/version conventions. |

The board owns its copper vectors exactly once. The grid type is chosen at compile time. Net-domain classification walks the original name without allocating uppercase text or a token vector. The UUID audit borrows each unique value and allocates output strings only for duplicates.
