use crate::structs::marker::{MarkerId, MarkerTarget, TargetState};

// One marker of the loaded world as the UI lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerRow {
    pub id: MarkerId,
    pub name: String,
    pub target: MarkerTarget,
    // The name of the structure or project the marker shows; empty when it is missing.
    pub target_name: String,
    pub state: TargetState,
    // Blocks from the player's feet to the marker, rounded.
    pub distance: i32,
    pub active: bool,
}
