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
| `undo`, `redo` | Restore or reapply a wiring, label or binding edit. Ctrl+Z and Ctrl+Shift+Z invoke the same history. |
| `erc` | Inspect missing connections and competing output pins. |
| `simulate`, `cursor 1` | Solve the compiled DC deck and read a selected voltage and signed pin-current pair. Cursor indices are zero-based. |
| `save`, `open` | Save or reopen the supplied project path. Opening discards unsaved edits. |
| `export` | Write the same deck used by the simulation beside the snapshot with a .cir extension. |

Simulation sweeps the first voltage source over 1, 2 and 3 V. The compiled deck uses stable numeric net IDs and model terminal order. The project catalog currently covers independent sources and resistors. The native replay sends window messages to the editor it starts, checks ERC and history, and compares all three post-reopen Iris cursor samples to the CLI.
