use crate::services::{StBoundary, StHandle};

/// True while the library keeps itself open through the file picker and the game must not open its pause
/// screen for the focus the picker took. The shim's focus-loss hook skips the game's handler while this is
/// true. False for a null handle or after a panic.
#[unsafe(no_mangle)]
pub extern "C" fn st_pause_screen_unwanted(handle: *mut StHandle) -> bool {
    StBoundary::guard_value(false, || {
        // SAFETY: the handle came from st_create, or is null
        let Some(handle) = (unsafe { StBoundary::handle(handle) }) else { return false };
        handle.with_state(|state| state.pause_screen_unwanted()).unwrap_or(false)
    })
}
