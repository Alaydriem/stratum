use stratum_core::structs::frame::FrameStatus;

use super::StStr;

// What the game reports on one tick. `world` names the loaded world and lives for the call; it may be empty
// outside a world. `time_seconds` is a steady clock that only moves forward. `render_distance` is the game's
// render distance in chunks.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StFrameStatus {
    pub in_world: bool,
    pub has_player: bool,
    pub menu_open: bool,
    pub world: StStr,
    pub dimension: i32,
    pub feet_x: i32,
    pub feet_y: i32,
    pub feet_z: i32,
    pub time_seconds: f64,
    pub render_distance: u32,
}

impl StFrameStatus {
    // The core status, with `world` the name the caller read from `self.world` as UTF-8, bad bytes replaced. The C view
    // cannot become a core type in a `From`, because its name is behind a raw pointer.
    pub fn to_core(&self, world: String) -> FrameStatus {
        FrameStatus {
            in_world: self.in_world,
            has_player: self.has_player,
            menu_open: self.menu_open,
            world,
            dimension: self.dimension,
            feet: [self.feet_x, self.feet_y, self.feet_z],
            time_seconds: self.time_seconds,
            render_distance: self.render_distance,
        }
    }
}
