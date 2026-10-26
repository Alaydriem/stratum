use std::path::Path;
use std::sync::Arc;

use amm_client::test_support::{FakeTable, LoggedCall};
use amm_core::structs::event::{KeybindPhase, ModEvent};
use stratum_ffi::api::lifecycle::st_destroy;
use stratum_ffi::api::tick::st_tick;
use stratum_ffi::services::{SharedUi, StHandle};
use stratum_ffi::types::{StFrameStatus, StStatus, StStr};
use tempfile::TempDir;

// A live handle registered on this thread's fake manager, with its own data folder. Destroyed on drop.
pub struct StSession {
    pub handle: *mut StHandle,
    pub data: TempDir,
}

impl StSession {
    pub const WORLD: &'static str = "Survival One";
    pub const OPEN_KEY: ModEvent = ModEvent::Keybind { action: 0, phase: KeybindPhase::Pressed };

    pub fn new() -> Self {
        FakeTable::set_config("");
        let data = TempDir::new().unwrap();
        let handle = StHandle::connect(FakeTable::table(), Path::new(FakeTable::CONFIG_DIR), data.path()).unwrap();
        Self { handle: Box::into_raw(Box::new(handle)), data }
    }

    pub fn str(text: &'static str) -> StStr {
        StStr { ptr: text.as_ptr(), len: text.len() }
    }

    // In the overworld of `WORLD`, no menu, feet at (10, 64, -20), render distance 12 chunks.
    pub fn in_world() -> StFrameStatus {
        StFrameStatus {
            in_world: true,
            has_player: true,
            menu_open: false,
            world: Self::str(Self::WORLD),
            dimension: 0,
            feet_x: 10,
            feet_y: 64,
            feet_z: -20,
            time_seconds: 1.0,
            render_distance: 12,
        }
    }

    // In the world at game time `time_seconds`.
    pub fn in_world_at(time_seconds: f64) -> StFrameStatus {
        StFrameStatus { time_seconds, ..Self::in_world() }
    }

    // In the world with the pause screen or another game menu shown.
    pub fn in_menu() -> StFrameStatus {
        StFrameStatus { menu_open: true, ..Self::in_world() }
    }

    // In the world at game time `time_seconds` with a game menu shown.
    pub fn in_menu_at(time_seconds: f64) -> StFrameStatus {
        StFrameStatus { menu_open: true, ..Self::in_world_at(time_seconds) }
    }

    pub fn out_of_world() -> StFrameStatus {
        StFrameStatus {
            in_world: false,
            has_player: false,
            world: StStr { ptr: std::ptr::null(), len: 0 },
            ..Self::in_world()
        }
    }

    pub fn tick(&self, status: &StFrameStatus) -> StStatus {
        st_tick(self.handle, status)
    }

    pub fn handle(&self) -> &StHandle {
        // SAFETY: the handle came from connect and lives until drop
        unsafe { &*self.handle }
    }

    pub fn shared(&self) -> Arc<SharedUi> {
        Arc::clone(self.handle().shared())
    }

    // The open key delivered on an in-world tick.
    pub fn open(&self) {
        FakeTable::push_event(Self::OPEN_KEY);
        assert_eq!(self.tick(&Self::in_world()), StStatus::Ok);
    }

    pub fn logged(fragment: &str) -> Vec<LoggedCall> {
        FakeTable::with(|state| state.logs.iter().filter(|call| call.message.contains(fragment)).cloned().collect())
    }

    // Destroys the handle now; the data folder stays until the session drops.
    pub fn destroy(&mut self) {
        st_destroy(self.handle);
        self.handle = std::ptr::null_mut();
    }
}

impl Drop for StSession {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            st_destroy(self.handle);
        }
    }
}
