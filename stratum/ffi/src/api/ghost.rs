use crate::services::{StBoundary, StHandle};
use crate::types::{StGhostFrame, StGhostReport, StStatus};

/// Fills `out` with this render frame's ghost work: drop every mesh and block lookup when `world_epoch` changed, and hold none while `in_world` is false,
/// then free every slot when `clear_all`, then free `frees`, then build `jobs`, then draw `draws`; resolve
/// `palette` again when `has_palette`. Call it once per rendered frame in a world, also when nothing is drawn, on the
/// render thread: the time between two calls is the render frame time in the telemetry. It never waits for the tick. Every pointer in `out` views a buffer the handle owns, valid until the next
/// st_ghost_take on the same handle or st_destroy. A take before the next tick gets the same draws and no clear,
/// frees, jobs or palette. Returns InvalidArgument for a null pointer, and Failed after an earlier panic.
#[unsafe(no_mangle)]
pub extern "C" fn st_ghost_take(handle: *mut StHandle, out: *mut StGhostFrame) -> StStatus {
    StBoundary::guard(|| {
        let (Some(handle), Some(out)) = (
            // SAFETY: the handle came from st_create or is null; GhostRenderer::frameIn passes RustBridge::handle(),
            // which is null before st_create and after st_destroy
            unsafe { StBoundary::handle(handle) },
            // SAFETY: out is null or a writable frame for the call; GhostRenderer::frameIn passes a StGhostFrame on its
            // stack
            unsafe { StBoundary::output(out) },
        ) else {
            return StStatus::InvalidArgument;
        };
        match handle.ghost().take() {
            Some(take) => {
                take.fill(out);
                StStatus::Ok
            }
            None => StStatus::Failed,
        }
    })
}

/// The shim's answer for the jobs it built and the frames it drew since the last report. `report` and its `built`
/// entries live for the call; each entry echoes its job's `slot`, `serial` and `generation`, with the mesh's GPU
/// bytes. Entries past the slot capacity are not read. The tick reads the report on its next run; a report sent
/// while 8 wait is folded into the newest, so no entry is lost, and a report sent while the tick is reading the
/// queue is itself dropped. Set `palette_resolved` on the report of the frame that resolved a palette, with that
/// palette's fallback counts, which are read only with `palette_resolved`. Set
/// `world_reset` on the first report after the shim dropped its meshes and block lookups: the next frame starts a
/// new generation with the palette and a clear. Returns InvalidArgument for a null pointer, and Failed after an
/// earlier panic.
#[unsafe(no_mangle)]
pub extern "C" fn st_ghost_report(handle: *mut StHandle, report: *const StGhostReport) -> StStatus {
    StBoundary::guard(|| {
        let (Some(handle), Some(report)) = (
            // SAFETY: the handle came from st_create or is null; GhostRenderer::frameIn passes RustBridge::handle()
            unsafe { StBoundary::handle(handle) },
            // SAFETY: report is null or readable for the call; GhostRenderer::frameIn passes a StGhostReport on its
            // stack
            unsafe { StBoundary::input(report) },
        ) else {
            return StStatus::InvalidArgument;
        };
        let count = handle.ghost().built_limit(report.built_count);
        // SAFETY: built points to at least built_count entries for the call; GhostRenderer::frameIn passes
        // mBuilt.data() with mBuilt.size(), a vector it does not touch until the call returns
        let Some(built) = (unsafe { StBoundary::slice(report.built, count) }) else {
            return StStatus::InvalidArgument;
        };
        match handle.ghost().report(report.world_reset, |core| report.fill(built, core)) {
            Some(()) => StStatus::Ok,
            None => StStatus::Failed,
        }
    })
}

/// A block of the loaded world changed at (x, y, z). The ghost section that holds it is rebuilt from the next
/// tick; changes past the tick's bound mark the whole build for a rebuild. Call it on the game thread. Returns
/// InvalidArgument for a null handle, and Failed after an earlier panic.
#[unsafe(no_mangle)]
pub extern "C" fn st_block_changed(handle: *mut StHandle, x: i32, y: i32, z: i32) -> StStatus {
    StBoundary::guard(|| {
        // SAFETY: the handle came from st_create or is null; BlockChangeHook passes RustBridge::handle() and only when
        // it is not null
        let Some(handle) = (unsafe { StBoundary::handle(handle) }) else { return StStatus::InvalidArgument };
        match handle.with_state(|state| state.block_changed([x, y, z])) {
            Some(()) => StStatus::Ok,
            None => StStatus::Failed,
        }
    })
}
