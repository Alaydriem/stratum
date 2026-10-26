use crate::services::StBoundary;
use crate::services::StHandle;
use crate::types::{StFrameStatus, StStatus};

/// Runs one game tick: the manager's events, the library UI's actions and the core's tick. Call it once per
/// client tick on the game thread. `status` and its `world` text live for the call. A world key that is not
/// UTF-8 is read with U+FFFD in place of the bad bytes. Returns InvalidArgument for a null pointer, and Failed
/// after an earlier panic.
#[unsafe(no_mangle)]
pub extern "C" fn st_tick(handle: *mut StHandle, status: *const StFrameStatus) -> StStatus {
    StBoundary::guard(|| {
        // SAFETY: the handle came from st_create and status is readable for the call
        let (Some(handle), Some(status)) =
            (unsafe { StBoundary::handle(handle) }, unsafe { StBoundary::input(status) })
        else {
            return StStatus::InvalidArgument;
        };
        // SAFETY: the world view points to bytes the shim owns for the call
        let Some(world) = (unsafe { StBoundary::text_lossy(&status.world) }) else { return StStatus::InvalidArgument };
        let status = status.to_core(world);
        match handle.with_state(|state| state.tick(&status)) {
            Some(()) => StStatus::Ok,
            None => StStatus::Failed,
        }
    })
}
