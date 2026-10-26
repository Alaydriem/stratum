use stratum_core::structs::ghost::GhostDraw;

// One built slot to draw this frame. The slot's mesh positions are relative to `origin`, a world cell. Draw the
// mesh moved by `offset` world cells and scaled by `scale` about `origin`: the full ghost has offset 0 and scale
// 1, the mini ghost has a scale below 1.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StGhostDraw {
    pub slot: u32,
    pub origin: [i32; 3],
    /// World cells added to the mesh position. The full ghost has 0.
    pub offset: [i32; 3],
    /// Factor applied to the mesh positions about `origin`. The full ghost and the marker have 1.0, the mini ghost less.
    pub scale: f32,
}

impl From<&GhostDraw> for StGhostDraw {
    fn from(draw: &GhostDraw) -> Self {
        Self { slot: draw.slot, origin: draw.origin, offset: draw.offset, scale: draw.scale }
    }
}
