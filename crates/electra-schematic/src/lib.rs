//! Editable schematic connectivity shared by editor and simulation front ends.
mod connectivity;
pub use connectivity::{
    Diagnostic, ModelBinding, NetId, PackageFinding, Pin, PinId, PinMap, PlacementError, Rewire,
    RewireError, Role, Schematic,
};
pub mod kicad;
