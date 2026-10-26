use stratum_ffi::api::ghost::st_ghost_take;
use stratum_ffi::types::{StGhostDraw, StGhostFrame, StGhostJob, StStatus};

use super::StSession;

// One st_ghost_take, copied out of the borrowed views so a test can read it after the next take.
#[derive(Debug, Clone)]
pub struct GhostSeen {
    pub status: StStatus,
    pub generation: u64,
    pub world_epoch: u64,
    pub in_world: bool,
    pub clear_all: bool,
    pub draws: Vec<StGhostDraw>,
    pub jobs: Vec<StGhostJob>,
    pub job_cells: Vec<Vec<u16>>,
    pub frees: Vec<u32>,
    // The NBT bytes of each entry, when the palette changed.
    pub palette: Option<Vec<Vec<u8>>>,
    pub opacity: f32,
    pub rotation: u8,
}

impl GhostSeen {
    // The first job that builds a full ghost section: the mini ghost and the marker skip no matching blocks.
    pub fn full_job(&self) -> &StGhostJob {
        self.jobs.iter().find(|job| job.skip_matching).expect("the take holds a full ghost job")
    }

    // The draws at scale 1.0: the full ghost, and the marker when it is drawn.
    pub fn unscaled_draws(&self) -> Vec<StGhostDraw> {
        self.draws.iter().filter(|draw| draw.scale == 1.0).copied().collect()
    }

    // The draws below scale 1.0: the mini ghost.
    pub fn miniature_draws(&self) -> Vec<StGhostDraw> {
        self.draws.iter().filter(|draw| draw.scale < 1.0).copied().collect()
    }

    pub fn take(session: &StSession) -> Self {
        let mut out = StGhostFrame::empty();
        let status = st_ghost_take(session.handle, &mut out);
        // SAFETY: every view in `out` points into the handle's buffers, which live until the next take
        unsafe { Self::copy(status, &out) }
    }

    // SAFETY: the caller passes a frame whose views are live
    unsafe fn copy(status: StStatus, out: &StGhostFrame) -> Self {
        // SAFETY: see the function contract
        let draws = unsafe { Self::slice(out.draws, out.draw_count as usize) }.to_vec();
        // SAFETY: see the function contract
        let jobs = unsafe { Self::slice(out.jobs, out.job_count as usize) }.to_vec();
        // SAFETY: each job's cells point to cell_count ids in the handle's buffers
        let job_cells = jobs.iter().map(|job| unsafe { Self::slice(job.cells, job.cell_count) }.to_vec()).collect();
        // SAFETY: see the function contract
        let frees = unsafe { Self::slice(out.frees, out.free_count as usize) }.to_vec();
        let palette = out.has_palette.then(|| {
            // SAFETY: see the function contract
            let entries = unsafe { Self::slice(out.palette, out.palette_count as usize) };
            // SAFETY: each entry's bytes live in the handle's buffers
            entries.iter().map(|entry| unsafe { Self::slice(entry.nbt.ptr, entry.nbt.len) }.to_vec()).collect()
        });
        Self {
            status,
            generation: out.generation,
            world_epoch: out.world_epoch,
            in_world: out.in_world,
            clear_all: out.clear_all,
            draws,
            jobs,
            job_cells,
            frees,
            palette,
            opacity: out.opacity,
            rotation: out.rotation,
        }
    }

    // SAFETY: ptr is null with len 0, or points to len live values
    unsafe fn slice<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
        if len == 0 {
            return &[];
        }
        // SAFETY: see the function contract
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}
