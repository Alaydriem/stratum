use stratum_core::structs::ghost::GhostJob;

use super::GhostOutbox;
use crate::types::{StBytes, StGhostBlock, StGhostDraw, StGhostFrame, StGhostJob};

// The render thread's copy of the last ghost frame it took, and the C views of it that st_ghost_take hands out.
// The views point into this value's own buffers and stay valid until the next take. Every buffer is reused, so a
// take allocates only while a buffer grows; a repeated take keeps the jobs it does not hand out, so the next
// receive gives their cell buffers back to the outbox.
//
// Bound: `draws`, `jobs` and `frees` hold one frame's worth, at most `slot_capacity` entries each; the palette is
// bounded by core's palette limit.
pub struct GhostTake {
    generation: u64,
    world_epoch: u64,
    in_world: bool,
    clear_all: bool,
    slot_capacity: u32,
    opacity: f32,
    // Clockwise quarter turns, 0 to 3.
    rotation: u8,
    draws: Vec<StGhostDraw>,
    jobs: Vec<(u64, GhostJob)>,
    // The jobs came with this take; false after a repeat, which hands out none.
    fresh_jobs: bool,
    job_views: Vec<StGhostJob>,
    frees: Vec<u32>,
    has_palette: bool,
    palette_bytes: Vec<u8>,
    palette_ends: Vec<usize>,
    palette_views: Vec<StGhostBlock>,
}

// SAFETY: the raw pointers in `job_views` and `palette_views` point into this value's own heap buffers, which do
// not move when the value moves between threads; the value is reached only through SharedGhost's lock
unsafe impl Send for GhostTake {}

impl GhostTake {
    pub fn new() -> Self {
        Self {
            generation: 0,
            world_epoch: 0,
            in_world: false,
            clear_all: false,
            slot_capacity: 0,
            opacity: 1.0,
            rotation: 0,
            draws: Vec::new(),
            jobs: Vec::new(),
            fresh_jobs: false,
            job_views: Vec::new(),
            frees: Vec::new(),
            has_palette: false,
            palette_bytes: Vec::new(),
            palette_ends: Vec::new(),
            palette_views: Vec::new(),
        }
    }

    // Takes everything the outbox holds.
    pub fn receive(&mut self, outbox: &mut GhostOutbox) {
        self.generation = outbox.generation();
        self.world_epoch = outbox.world_epoch();
        self.in_world = outbox.in_world();
        self.clear_all = outbox.clear_all();
        self.slot_capacity = outbox.slot_capacity();
        self.opacity = outbox.opacity();
        self.rotation = outbox.rotation().quarters();
        self.draws.clear();
        self.draws.extend(outbox.draws().iter().map(StGhostDraw::from));
        self.has_palette =
            outbox.hand_over(&mut self.jobs, &mut self.frees, &mut self.palette_bytes, &mut self.palette_ends);
        self.fresh_jobs = true;
        self.build_views();
    }

    // Nothing new since the last take: the same draws, and no clear, frees, jobs or palette.
    pub fn repeat(&mut self) {
        self.clear_all = false;
        self.fresh_jobs = false;
        self.frees.clear();
        self.has_palette = false;
        self.job_views.clear();
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn world_epoch(&self) -> u64 {
        self.world_epoch
    }

    pub fn in_world(&self) -> bool {
        self.in_world
    }

    pub fn clear_all(&self) -> bool {
        self.clear_all
    }

    pub fn slot_capacity(&self) -> u32 {
        self.slot_capacity
    }

    pub fn draws(&self) -> &[StGhostDraw] {
        &self.draws
    }

    // Each job with the generation of the frame that gave it.
    pub fn jobs(&self) -> &[(u64, GhostJob)] {
        if self.fresh_jobs { &self.jobs } else { &[] }
    }

    pub fn frees(&self) -> &[u32] {
        &self.frees
    }

    // The NBT bytes of each palette entry when the palette changed in this take.
    pub fn palette(&self) -> Option<impl Iterator<Item = &[u8]>> {
        self.has_palette.then(|| {
            let mut start = 0;
            self.palette_ends.iter().map(move |end| {
                let entry = &self.palette_bytes[start..*end];
                start = *end;
                entry
            })
        })
    }

    // Points `out` at this take's views.
    pub fn fill(&self, out: &mut StGhostFrame) {
        let palette_count = if self.has_palette { self.palette_views.len() } else { 0 };
        *out = StGhostFrame {
            generation: self.generation,
            world_epoch: self.world_epoch,
            in_world: self.in_world,
            clear_all: self.clear_all,
            slot_capacity: self.slot_capacity,
            draws: self.draws.as_ptr(),
            draw_count: Self::count(self.draws.len()),
            jobs: self.job_views.as_ptr(),
            job_count: Self::count(self.job_views.len()),
            frees: self.frees.as_ptr(),
            free_count: Self::count(self.frees.len()),
            has_palette: self.has_palette,
            palette: self.palette_views.as_ptr(),
            palette_count: Self::count(palette_count),
            opacity: self.opacity,
            rotation: self.rotation,
        };
    }

    fn count(len: usize) -> u32 {
        u32::try_from(len).unwrap_or(u32::MAX)
    }

    fn build_views(&mut self) {
        self.job_views.clear();
        self.job_views.extend(self.jobs.iter().map(|(generation, job)| StGhostJob {
            slot: job.slot,
            serial: job.serial,
            generation: *generation,
            origin: job.origin,
            dims: job.dims,
            cells: job.cells.as_ptr(),
            cell_count: job.cells.len(),
            skip_matching: job.skip_matching,
            mode: job.mode.into(),
            group_serial: job.group_serial,
            group_index: job.group_index,
            group_offset: job.group_offset,
            slot_size: job.slot_size,
        }));
        if !self.has_palette {
            return;
        }
        self.palette_views.clear();
        let mut start = 0;
        for end in &self.palette_ends {
            let nbt = StBytes::of(&self.palette_bytes[start..*end]);
            self.palette_views.push(StGhostBlock { nbt });
            start = *end;
        }
    }
}

impl Default for GhostTake {
    fn default() -> Self {
        Self::new()
    }
}
