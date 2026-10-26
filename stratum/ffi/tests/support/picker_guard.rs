use stratum_core::structs::action::LibraryAction;
use stratum_ffi::api::pause::st_pause_screen_unwanted;
use stratum_ffi::api::picker::{st_files_picked, st_take_picker_request};

use super::StSession;

// Drives the picker the way the shim and the library do.
pub struct PickerGuard;

impl PickerGuard {
    // The library asked for the picker on an in-world tick at `time` and the shim opened it.
    pub fn start(session: &StSession, time: f64) {
        session.shared().send(&mut vec![LibraryAction::RequestFilePicker(None)]);
        session.tick(&StSession::in_world_at(time));
        assert!(st_take_picker_request(session.handle));
    }

    // The picker was cancelled: the controller is idle again.
    pub fn cancel(session: &StSession) {
        st_files_picked(session.handle, std::ptr::null(), 0);
    }

    pub fn unwanted(session: &StSession) -> bool {
        st_pause_screen_unwanted(session.handle)
    }
}
