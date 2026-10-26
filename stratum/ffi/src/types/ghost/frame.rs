use super::{StGhostBlock, StGhostDraw, StGhostJob};

// One render frame of the ghost, as borrowed views that stay valid until the next st_ghost_take. The shim
// applies it in this order: `clear_all` frees every slot, then `frees`, then `jobs`, then `draws`.
// `world_epoch` moves each time core sees the world change or the player leave it: the shim then drops its meshes
// and block lookups before it applies the frame; `in_world` is false between two worlds, when the shim holds nothing. `has_palette` says the palette changed: the shim resolves
// `palette` again, indexed by palette id.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StGhostFrame {
    pub generation: u64,
    pub world_epoch: u64,
    pub in_world: bool,
    pub clear_all: bool,
    pub slot_capacity: u32,
    pub draws: *const StGhostDraw,
    pub draw_count: u32,
    pub jobs: *const StGhostJob,
    pub job_count: u32,
    pub frees: *const u32,
    pub free_count: u32,
    pub has_palette: bool,
    pub palette: *const StGhostBlock,
    pub palette_count: u32,
    // 0 to 1, the alpha of every ghost vertex
    pub opacity: f32,
    // 0 to 3 clockwise quarter turns seen from above, as Minecraft's Rotate90: the shim turns each block state by it
    pub rotation: u8,
}

impl StGhostFrame {
    // A frame with nothing in it, for the caller to pass as the out parameter.
    pub fn empty() -> Self {
        Self {
            generation: 0,
            world_epoch: 0,
            in_world: false,
            clear_all: false,
            slot_capacity: 0,
            draws: std::ptr::null(),
            draw_count: 0,
            jobs: std::ptr::null(),
            job_count: 0,
            frees: std::ptr::null(),
            free_count: 0,
            has_palette: false,
            palette: std::ptr::null(),
            palette_count: 0,
            opacity: 0.0,
            rotation: 0,
        }
    }
}
