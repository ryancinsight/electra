//! Editable schematic connectivity shared by editor and simulation front ends.
mod connectivity;
pub use connectivity::{
    Diagnostic, NetId, PackageFinding, Pin, PinId, PinMap, Rewire, RewireError, Role, Schematic,
};
