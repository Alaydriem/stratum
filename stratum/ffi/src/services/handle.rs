use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use amm_client::link::ApiTable;
use amm_core::errors::LinkError;
use stratum_core::services::StratumController;
use stratum_core::services::preview::PreviewService;
use stratum_core::structs::preview::PreviewSource;

use super::{HandleState, ManagerSession, SharedGhost, SharedUi};

// The opaque handle C++ holds. The game thread reaches the state through the lock; the render thread reaches
// only `shared`, through the frame handler the session registered, and `ghost`, through st_ghost_take and
// st_ghost_report.
pub struct StHandle {
    state: Mutex<HandleState>,
    shared: Arc<SharedUi>,
    ghost: Arc<SharedGhost>,
    is_poison_reported: AtomicBool,
}

impl StHandle {
    // Registers with the manager and starts the controller from the manager's config; an unreadable config
    // gives the defaults. `config_dir` is the mod's LeviLamina config folder and `data_dir` the folder for the
    // library, both absolute paths.
    pub fn connect(table: ApiTable, config_dir: &Path, data_dir: &Path) -> Result<Self, LinkError> {
        let source = PreviewSource::locate(data_dir);
        let preview = PreviewService::new_shared(source);
        let shared = SharedUi::new_shared_with_preview(preview);
        let session = ManagerSession::connect(table, config_dir, Arc::clone(&shared))?;
        let config = session.config().unwrap_or_default();
        shared.meter().set_enabled(config.debug.telemetry);
        let controller = StratumController::new(data_dir.to_path_buf(), config);
        let ghost = SharedGhost::new_shared();
        let state = Mutex::new(HandleState::new(controller, session, Arc::clone(&shared), Arc::clone(&ghost)));
        Ok(Self { state, shared, ghost, is_poison_reported: AtomicBool::new(false) })
    }

    // None when a panic poisoned the lock. The first such call logs it and gives back the library's leases and
    // the cursor, so the player is not left without turning until the game exits; later calls stay quiet.
    pub fn with_state<R>(&self, body: impl FnOnce(&mut HandleState) -> R) -> Option<R> {
        match self.state.lock() {
            Ok(mut state) => Some(body(&mut state)),
            Err(poisoned) => {
                if !self.is_poison_reported.swap(true, Ordering::Relaxed) {
                    curia::error!("stratum lock is poisoned, the library stops until the game restarts", {});
                    let mut state = poisoned.into_inner();
                    if catch_unwind(AssertUnwindSafe(|| state.release_ui())).is_err() {
                        curia::error!("library leases not given back after the panic", {});
                    }
                }
                None
            }
        }
    }

    // The state the frame handler shares with the tick.
    pub fn shared(&self) -> &Arc<SharedUi> {
        &self.shared
    }

    // The ghost frame and reports the render thread shares with the tick.
    pub fn ghost(&self) -> &Arc<SharedGhost> {
        &self.ghost
    }

    // Stops the controller within its budget, then unregisters from the manager. A poisoned lock still drops
    // the state, so the registration ends either way.
    pub fn shutdown(self) {
        let mut state = match self.state.into_inner() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
        state.shutdown();
        drop(state);
    }
}
