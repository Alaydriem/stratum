use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::services::StHandle;
use crate::types::{StStatus, StStr};

// The rules every entry point follows: no panic crosses into C++, every pointer is checked.
pub struct StBoundary;

impl StBoundary {
    // Runs the body; a panic becomes StStatus::Panicked and a log line.
    pub fn guard(body: impl FnOnce() -> StStatus) -> StStatus {
        Self::guard_value(StStatus::Panicked, body)
    }

    // Like guard, for entry points that return a value; `fallback` is returned on panic.
    pub fn guard_value<T>(fallback: T, body: impl FnOnce() -> T) -> T {
        match catch_unwind(AssertUnwindSafe(body)) {
            Ok(value) => value,
            Err(payload) => {
                let message = payload
                    .downcast_ref::<&str>()
                    .map(|text| (*text).to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic".to_string());
                curia::error!("panic stopped at the C boundary", { payload: message });
                fallback
            }
        }
    }

    // SAFETY: the caller passes either null or a pointer returned by st_create and not yet destroyed
    pub unsafe fn handle<'a>(handle: *const StHandle) -> Option<&'a StHandle> {
        // SAFETY: see the function contract
        unsafe { handle.as_ref() }
    }

    // SAFETY: the caller passes either null or a pointer to a live, aligned T for the call
    pub unsafe fn input<'a, T>(value: *const T) -> Option<&'a T> {
        // SAFETY: see the function contract
        unsafe { value.as_ref() }
    }

    // SAFETY: the caller passes either null or a pointer to a live, aligned T it lets us write for the call
    pub unsafe fn output<'a, T>(value: *mut T) -> Option<&'a mut T> {
        // SAFETY: see the function contract
        unsafe { value.as_mut() }
    }

    // None when ptr is null and len is not 0; an empty slice when len is 0.
    // SAFETY: the caller passes either null or a pointer to len live, aligned T for the call
    pub unsafe fn slice<'a, T>(ptr: *const T, len: usize) -> Option<&'a [T]> {
        if len == 0 {
            return Some(&[]);
        }
        if ptr.is_null() {
            return None;
        }
        // SAFETY: see the function contract
        Some(unsafe { std::slice::from_raw_parts(ptr, len) })
    }

    // SAFETY: the view is null with len 0, or points to len readable bytes for the call
    pub unsafe fn text(value: &StStr) -> Option<String> {
        // SAFETY: see the function contract
        let bytes = unsafe { Self::slice(value.ptr, value.len) }?;
        std::str::from_utf8(bytes).ok().map(str::to_string)
    }

    // Like `text`, but bytes that are not UTF-8 become U+FFFD instead of failing. None only for a null view
    // with a length.
    // SAFETY: the view is null with len 0, or points to len readable bytes for the call
    pub unsafe fn text_lossy(value: &StStr) -> Option<String> {
        // SAFETY: see the function contract
        let bytes = unsafe { Self::slice(value.ptr, value.len) }?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}
