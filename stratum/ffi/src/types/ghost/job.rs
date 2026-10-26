use super::StGhostJobMode;

// One section mesh to build into `slot`. `cells` holds `cell_count` palette ids, the padded grid of `dims + 2`
// cells per axis, X fastest, then Z, then Y, whose minimum cell is `origin - 1`; `u16::MAX` is empty. Mesh
// positions are relative to `origin`. The report for the job echoes `serial` and `generation`. With
// `skip_matching` the shim leaves out a cell whose world block has the same name as its ghost block. `mode`,
// `group_serial`, `group_index` and `group_offset` describe how the job joins the slot mesh of its group; `slot_size`
// is the size in cells of the box the slot draws.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StGhostJob {
    pub slot: u32,
    pub serial: u32,
    pub generation: u64,
    pub origin: [i32; 3],
    pub dims: [u32; 3],
    pub cells: *const u16,
    pub cell_count: usize,
    pub skip_matching: bool,
    pub mode: StGhostJobMode,
    pub group_serial: u32,
    pub group_index: u32,
    pub group_offset: [i32; 3],
    pub slot_size: [u32; 3],
}
