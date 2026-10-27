use std::time::{Duration, Instant};

use amm_client::test_support::FakeTable;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use stratum_ffi::api::easy_place::st_use_aim;
use stratum_ffi::types::{StInventorySlot, StStatus, StStr, StUseProbe};

use super::{GhostScene, StSession};

// Steps for tests that place through the use path: the ghost scene with easy place switched on from the HUD keys.
pub struct EasyPlace;

impl EasyPlace {
    const DEADLINE: Duration = Duration::from_secs(60);
    pub const STONE: &'static str = "minecraft:stone";

    // The ghost scene, with the Mode, Left and Up keys pressed: Full mode, the Easy place control, toggle.
    pub fn start() -> StSession {
        let (session, _seen) = GhostScene::start();
        for action in [5, 3, 1] {
            FakeTable::push_event(ModEvent::Keybind { action, phase: KeybindPhase::Pressed });
            assert_eq!(session.tick(&StSession::in_world()), StStatus::Ok);
            FakeTable::push_event(ModEvent::Keybind { action, phase: KeybindPhase::Released });
        }
        assert_eq!(session.tick(&StSession::in_world()), StStatus::Ok);
        session
    }

    // Looks straight down at the column (x, z) from above the build.
    pub fn above(x: i32, z: i32) -> StUseProbe {
        StUseProbe {
            time_seconds: 1.0,
            pressed: false,
            held: true,
            eye: [f64::from(x) + 0.5, 70.5, f64::from(z) + 0.5],
            dir: [0.0, -1.0, 0.0],
            reach: 20.0,
            game_mode: 0,
            has_hit: false,
            hit_cell: [0; 3],
            hit_face: 0,
            hit_distance: 0.0,
            hit_replaceable: false,
            has_body: false,
            body_min: [0; 3],
            body_max: [0; 3],
            swap_answer: 0,
        }
    }

    pub fn stone_in_hand() -> [StInventorySlot; 1] {
        [StInventorySlot { item: StStr { ptr: Self::STONE.as_ptr(), len: Self::STONE.len() }, count: 1 }]
    }

    // Ticks until the aim at `probe` names a spot, and returns it.
    pub fn aim_when_ready(session: &StSession, probe: &StUseProbe) -> [i32; 3] {
        let start = Instant::now();
        loop {
            assert_eq!(session.tick(&StSession::in_world()), StStatus::Ok);
            let (mut spot, mut has_spot) = ([0; 3], false);
            assert_eq!(st_use_aim(session.handle, probe, &mut spot, &mut has_spot), StStatus::Ok);
            if has_spot {
                return spot;
            }
            assert!(start.elapsed() < Self::DEADLINE, "the ghost never gave a spot");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}
