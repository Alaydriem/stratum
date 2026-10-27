use stratum_core::structs::easy_place::{InventorySlot, SpotWorld, UseProbe};

use crate::services::{StBoundary, StHandle};
use crate::types::{StInventorySlot, StSpotWorld, StStatus, StStr, StUseCommand, StUseProbe};

/// Step one of a use call, on the game thread, while the use key is down: asks for the ghost spot under the
/// crosshair. Sets `*has_spot` and, only when it is true, writes the spot to `*spot_out`; then read the
/// blocks around the spot and call st_use_decide with the same probe. `probe`, `spot_out` and `has_spot` are
/// read or written during the call only. There is never a spot while the HUD keys are not live: no active
/// marker, a game screen or the library open, or another mod holding Input. Returns InvalidArgument for a null
/// pointer, and Failed after an earlier panic.
#[unsafe(no_mangle)]
pub extern "C" fn st_use_aim(
    handle: *mut StHandle,
    probe: *const StUseProbe,
    spot_out: *mut [i32; 3],
    has_spot: *mut bool,
) -> StStatus {
    StBoundary::guard(|| {
        let (Some(handle), Some(probe), Some(spot_out), Some(has_spot)) = (
            // SAFETY: the handle came from st_create or is null; the use hook passes RustBridge::handle()
            unsafe { StBoundary::handle(handle) },
            // SAFETY: probe is null or readable for the call; the use hook passes a StUseProbe on its stack
            unsafe { StBoundary::input(probe) },
            // SAFETY: spot_out is null or writable for the call; the use hook passes an int32_t[3] on its stack
            unsafe { StBoundary::output(spot_out) },
            // SAFETY: has_spot is null or writable for the call; the use hook passes a bool on its stack
            unsafe { StBoundary::output(has_spot) },
        ) else {
            return StStatus::InvalidArgument;
        };
        match handle.with_state(|state| state.use_aim(&UseProbe::from(probe))) {
            Some(spot) => {
                *has_spot = spot.is_some();
                if let Some(spot) = spot {
                    *spot_out = spot;
                }
                StStatus::Ok
            }
            None => StStatus::Failed,
        }
    })
}

/// Step two of a use call, on the game thread: asks what to do with this use. `probe` is the one given to
/// st_use_aim. `world` is null when st_use_aim gave no spot, otherwise what surrounds that spot. `slots` points
/// to `count` inventory slots, at most 36, slots 0 to 8 being the hotbar, and `selected` is the selected slot.
/// Every pointer, including each slot's item name, is read during the call only; `out` is written during the
/// call. Gives kind 0 (vanilla) while the HUD keys are not live. Returns InvalidArgument for a null handle,
/// probe, slots with a count, or out, a count above 36 or an item view that is null with a length, and Failed
/// after an earlier panic.
#[unsafe(no_mangle)]
pub extern "C" fn st_use_decide(
    handle: *mut StHandle,
    probe: *const StUseProbe,
    world: *const StSpotWorld,
    slots: *const StInventorySlot,
    count: u32,
    selected: u8,
    out: *mut StUseCommand,
) -> StStatus {
    StBoundary::guard(|| {
        let (Some(handle), Some(probe), Some(out)) = (
            // SAFETY: the handle came from st_create or is null; the use hook passes RustBridge::handle()
            unsafe { StBoundary::handle(handle) },
            // SAFETY: probe is null or readable for the call; the use hook passes a StUseProbe on its stack
            unsafe { StBoundary::input(probe) },
            // SAFETY: out is null or writable for the call; the use hook passes a StUseCommand on its stack
            unsafe { StBoundary::output(out) },
        ) else {
            return StStatus::InvalidArgument;
        };
        // SAFETY: world is null or readable for the call; the use hook passes null or a StSpotWorld on its stack
        let world = unsafe { StBoundary::input(world) }.map(SpotWorld::from);
        if count as usize > StInventorySlot::CAPACITY {
            return StStatus::InvalidArgument;
        }
        // SAFETY: slots is null with count 0, or points to count slots the use hook does not touch until the call
        // returns
        let Some(raw) = (unsafe { StBoundary::slice(slots, count as usize) }) else {
            return StStatus::InvalidArgument;
        };
        let mut stacks = [InventorySlot { item: &[], count: 0 }; StInventorySlot::CAPACITY];
        // SAFETY: each slot's item view is null with len 0, or points to len bytes of an item name the use hook
        // keeps alive for the call
        let Some(filled) = (unsafe { StInventorySlot::fill(raw, &mut stacks) }) else {
            return StStatus::InvalidArgument;
        };
        let probe = UseProbe::from(probe);
        match handle.with_state(|state| state.use_decide(&probe, world.as_ref(), &stacks[..filled], selected)) {
            Some(command) => {
                *out = StUseCommand::from(command);
                StStatus::Ok
            }
            None => StStatus::Failed,
        }
    })
}

/// The use key went up. Ends the hold: the spots tried in it are forgotten. Call it on the game thread. Returns
/// InvalidArgument for a null handle, and Failed after an earlier panic.
#[unsafe(no_mangle)]
pub extern "C" fn st_use_released(handle: *mut StHandle) -> StStatus {
    StBoundary::guard(|| {
        // SAFETY: the handle came from st_create or is null; the use hook passes RustBridge::handle()
        let Some(handle) = (unsafe { StBoundary::handle(handle) }) else { return StStatus::InvalidArgument };
        match handle.with_state(|state| state.use_released()) {
            Some(()) => StStatus::Ok,
            None => StStatus::Failed,
        }
    })
}

/// The block at (x, y, z) is `name` now, as the game reports it; the placements that wait for the server are
/// checked against it. Call it on the game thread for each block change. `name` is read during the call only.
/// `time_seconds` is the frame clock. Returns InvalidArgument for a null handle or a name view that is null with
/// a length, and Failed after an earlier panic.
#[unsafe(no_mangle)]
pub extern "C" fn st_block_seen(
    handle: *mut StHandle,
    x: i32,
    y: i32,
    z: i32,
    name: StStr,
    time_seconds: f64,
) -> StStatus {
    StBoundary::guard(|| {
        // SAFETY: the handle came from st_create or is null; the block hook passes RustBridge::handle()
        let Some(handle) = (unsafe { StBoundary::handle(handle) }) else { return StStatus::InvalidArgument };
        // SAFETY: the view is null with len 0, or points to len bytes the block hook keeps alive for the call
        let Some(block) = (unsafe { StBoundary::slice(name.ptr, name.len) }) else {
            return StStatus::InvalidArgument;
        };
        match handle.with_state(|state| state.block_seen([x, y, z], block, time_seconds)) {
            Some(()) => StStatus::Ok,
            None => StStatus::Failed,
        }
    })
}
