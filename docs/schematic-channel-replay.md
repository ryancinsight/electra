# Reusable channel replay

Run the two-channel acceptance fixture from the repository root with the Rust 1.97 toolchain required by the Metis adapter.

```sh
cargo +1.97.0 test --locked --manifest-path integrations/metis-probe/Cargo.toml --test channel
```

The fixture authors one packaged-device and load block and places it twice. Component occurrence IDs and private output nets differ across placements. Both channels bind the same supply and ground. Placement preflights the bindings so a collision leaves the destination schematic intact.

The package audit identifies both swapped pads and the missing supply terminal in channel B. After repair, the fixture saves the connectivity snapshot and reopens it. A Metis pointer click selects B, then the decoded Up key selects A. The selected row resolves through Electra's stable pin ID to Iris voltage and terminal-current series.

| Selected net | Source drive | Output RMS voltage | In-phase load current |
|---|---:|---:|---:|
| channel/B/OUT | 4 V RMS | 2 V | 2 mA |
| channel/A/OUT | 2 V RMS | 1 V | 1 mA |

The comparison uses a 1 kΩ series resistance and a 1 mS resistive load at 1 kHz. The test supplies the circuit model and result bindings explicitly. It exercises the Metis input adapter in process. Desktop window event delivery, schematic-to-model compilation and project-level persistence of model bindings are separate qualification work.
