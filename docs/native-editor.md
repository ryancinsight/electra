# Native schematic editor

The Windows workbench uses the pinned Metis window event loop and software framebuffer. Start it with a project snapshot path.

```powershell
cargo +1.97.0 run --locked --manifest-path integrations/metis-probe/Cargo.toml --bin electra-editor -- channels.elcp
```

Type a command and press Enter. Click a pin or move the focus with the Up and Down keys. Each placed channel has a voltage source, a series resistor and a grounded load. Component and pin IDs identify the selection independently of its label.

| Command | Effect |
|---|---|
| `place A` | Place a channel block with private A/VIN and A/OUT nets and shared ground. Placement starts a new undo baseline. |
| `wire 2` or `wire off` | Connect the focused terminal to a project net or disconnect it. |
| `label A/TAP` | Set the connected net's display name. |
| `bind R2k 1 2` | Assign an ordered two-pin DC model to the focused component. The catalog includes V2, R1k and R2k. |
| `inspect` | Show the selected sheet, component, pin and ordered model binding. |
| `bus taps 4 2`, `lanes taps` | Define an ordered logical group of net IDs or inspect its lanes. Declaring a bus leaves the nets electrically distinct. |
| `junction 2` | List all terminals electrically joined to net 2 in stable component and pin order. |
| `undo`, `redo` | Restore or reapply a wiring, bus, label or binding edit. Ctrl+Z and Ctrl+Shift+Z invoke the same history. |
| `erc` | Inspect missing connections and competing output pins. |
| `simulate`, `cursor 1` | Solve the compiled DC deck and read a selected voltage and signed pin-current pair. Cursor indices are zero-based. |
| `save`, `open` | Save or reopen the supplied project path. Opening discards unsaved edits. |
| `export` | Write the same deck used by the simulation beside the snapshot with a .cir extension. |

Simulation sweeps the first voltage source over 1, 2 and 3 V. The compiled deck uses stable numeric net IDs and model terminal order. The project catalog currently covers independent sources and resistors. The native replay sends window messages to the editor it starts, checks ERC and history, and compares all three post-reopen Iris cursor samples to the CLI.

The editor draws the selected voltage and signed current in separate panels against the same drive axis. The renderer and the numeric cursor borrow the same typed Iris series. Each panel spans its own minimum and maximum in the indicated units.

ELCP v4 stores a sheet path once for its group of parts and preserves the declared bus lane order. Electrical junctions derive from the stored pin-to-net connections. The reader also opens v1, v2 and v3 files. Older projects retain their existing part and net identities. Their original files did not store sheet membership.

## Placement measurements

Run `cargo run --locked --release -p electra-schematic --example sheet_scale` to place an existing hierarchy of 1,024 sheets and 8,192 parts under one new parent. On Linux x86-64 (Intel Xeon Platinum 8573C), 11 release processes gave a median placement time of 5.454 ms, with a 5.083–6.310 ms range. Median whole-process peak RSS was 3,144 KiB as measured by `wait4`. The placement timer excludes fixture construction and verification. RSS includes both source and destination projects. The fixture contains one unwired pin per part. Wired bus remapping and DC solving are covered by separate functional tests.
