use crate::services::{StBoundary, StHandle};
use crate::types::{PathList, StStatus, StStr};

/// True once after the library asked for the file picker. The shim then opens the picker and answers with
/// st_files_picked. False for a null handle, after a panic, or when nothing was asked.
#[unsafe(no_mangle)]
pub extern "C" fn st_take_picker_request(handle: *mut StHandle) -> bool {
    StBoundary::guard_value(false, || {
        // SAFETY: the handle came from st_create, or is null
        let Some(handle) = (unsafe { StBoundary::handle(handle) }) else { return false };
        handle.with_state(|state| state.take_picker_request()).unwrap_or(false)
    })
}

/// The picker's answer: `count` UTF-8 paths at `paths`, each living for the call. A `count` of 0 means the
/// picker was cancelled. The files are copied into the library; the sources stay where they are. When any
/// path is not UTF-8 nothing is imported and InvalidArgument is returned.
#[unsafe(no_mangle)]
pub extern "C" fn st_files_picked(handle: *mut StHandle, paths: *const StStr, count: u32) -> StStatus {
    StBoundary::guard(|| {
        // SAFETY: the handle came from st_create; paths points to count views the shim owns for the call
        let (Some(handle), Some(entries)) =
            (unsafe { StBoundary::handle(handle) }, unsafe { StBoundary::slice(paths, count as usize) })
        else {
            return StStatus::InvalidArgument;
        };
        // SAFETY: each view points to bytes the shim owns for the call
        let Some(list) = (unsafe { PathList::read(entries) }) else { return StStatus::InvalidArgument };
        match handle.with_state(|state| state.files_picked(list.into_paths())) {
            Some(()) => StStatus::Ok,
            None => StStatus::Failed,
        }
    })
}
