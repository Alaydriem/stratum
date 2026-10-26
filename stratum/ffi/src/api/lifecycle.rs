use std::path::Path;

use amm_client::link::ApiTable;

use crate::services::{StBoundary, StHandle, StLogLine};
use crate::types::{StStatus, StStr};

/// Registers Stratum with the mod manager, starts its core from the manager's config.hcl and places the pause
/// menu entry. `config_dir` is the mod's own LeviLamina config folder and `data_dir` the folder that holds the
/// library, both absolute UTF-8 paths that live for the call. Returns null when a path is null or not UTF-8,
/// when the mod manager is not loaded, or when it refuses the mod. Call it from the mod's enable, never under
/// the OS loader lock.
#[unsafe(no_mangle)]
pub extern "C" fn st_create(config_dir: *const StStr, data_dir: *const StStr) -> *mut StHandle {
    StBoundary::guard_value(std::ptr::null_mut(), || {
        // SAFETY: the shim passes pointers to StStr views it owns for the call, or null
        let (Some(config_dir), Some(data_dir)) =
            (unsafe { StBoundary::input(config_dir) }, unsafe { StBoundary::input(data_dir) })
        else {
            return std::ptr::null_mut();
        };
        // SAFETY: each view points to bytes the shim owns for the call
        let (Some(config_dir), Some(data_dir)) =
            (unsafe { StBoundary::text(config_dir) }, unsafe { StBoundary::text(data_dir) })
        else {
            return std::ptr::null_mut();
        };
        let Ok(table) = ApiTable::resolve() else { return std::ptr::null_mut() };
        match StHandle::connect(table, Path::new(&config_dir), Path::new(&data_dir)) {
            Ok(handle) => Box::into_raw(Box::new(handle)),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

/// Saves the library within a bounded time, stops the workers, unregisters from the mod manager and frees the
/// handle. Unhook first, so no other call is in flight. The handle must not be used again. Null is ignored.
#[unsafe(no_mangle)]
pub extern "C" fn st_destroy(handle: *mut StHandle) {
    let _ = StBoundary::guard(|| {
        if handle.is_null() {
            return StStatus::InvalidArgument;
        }
        // SAFETY: a non-null handle came from st_create and the shim destroys it exactly once
        let handle = unsafe { Box::from_raw(handle) };
        handle.shutdown();
        StStatus::Ok
    });
}

/// Sends one shim log line to the mod manager's log. `level` is an `AmmLogLevel` value and `fields_json` a JSON
/// object; each text is UTF-8 and lives for the call. Nothing is sent before st_create succeeds or after
/// st_destroy.
#[unsafe(no_mangle)]
pub extern "C" fn st_log(
    level: u8,
    msg: *const u8,
    msg_len: usize,
    fields_json: *const u8,
    fields_len: usize,
    file: *const u8,
    file_len: usize,
    line: u32,
) {
    let _ = StBoundary::guard(|| {
        let texts = [
            StStr { ptr: msg, len: msg_len },
            StStr { ptr: fields_json, len: fields_len },
            StStr { ptr: file, len: file_len },
        ];
        // SAFETY: the shim passes the bytes of live string views for the call
        let [Some(message), Some(fields), Some(file)] = texts.map(|text| unsafe { StBoundary::text(&text) }) else {
            return StStatus::InvalidArgument;
        };
        StLogLine::emit(level, &message, &fields, &file, line);
        StStatus::Ok
    });
}
