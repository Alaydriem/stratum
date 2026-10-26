use super::StSlotBuilt;
use stratum_core::structs::ghost::{GhostReport, SlotBuilt};

// What the shim sends back after a frame: one entry per job it ran (`built`, `built_count` entries that live for
// the call); when it resolved a palette in this frame, `palette_resolved` with the block lookups of that palette
// that fell back; the time the frame spent building, the sum of its jobs; the time spent drawing and its slowest
// draw; the meshes it drew; the cells built and the cells skipped as already in the world; the drawn slots it
// left out as outside the view; and `world_reset` when it dropped its meshes and block lookups for a new world.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StGhostReport {
    pub built: *const StSlotBuilt,
    pub built_count: u32,
    pub unknown_blocks: u32,
    pub default_state_blocks: u32,
    pub palette_resolved: bool,
    pub build_us: u64,
    pub draw_us_sum: u64,
    pub draw_frames: u32,
    pub draw_us_max: u64,
    pub meshes_drawn: u32,
    pub cells_built: u32,
    pub skipped_cells: u32,
    pub culled: u32,
    pub world_reset: bool,
}

impl StGhostReport {
    // Writes this report into `report`, reusing its buffer. `built` holds the entries the caller read from
    // `self.built`; the C view cannot become a core type in a `From`, because its entries are behind a raw pointer.
    pub fn fill(&self, built: &[StSlotBuilt], report: &mut GhostReport) {
        report.built.clear();
        report.built.extend(built.iter().map(SlotBuilt::from));
        report.unknown_blocks = self.unknown_blocks;
        report.default_state_blocks = self.default_state_blocks;
        report.palette_resolved = self.palette_resolved;
        report.build_us = self.build_us;
        report.draw_us_sum = self.draw_us_sum;
        report.draw_frames = self.draw_frames;
        report.draw_us_max = self.draw_us_max;
        report.meshes_drawn = self.meshes_drawn;
        report.cells_built = self.cells_built;
        report.skipped_cells = self.skipped_cells;
        report.culled = self.culled;
        report.world_reset = self.world_reset;
    }
}
