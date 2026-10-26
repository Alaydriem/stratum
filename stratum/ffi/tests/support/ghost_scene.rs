use std::time::{Duration, Instant};

use stratum_core::services::NbtWriter;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::marker::MarkerTarget;
use stratum_core::structs::nbt::{NbtCompound, NbtTag};
use stratum_ffi::api::ghost::st_ghost_report;
use stratum_ffi::api::picker::st_files_picked;
use stratum_ffi::types::{StGhostReport, StSlotBuilt, StStatus, StStr};

use super::{GhostSeen, StSession};

// Steps for tests that draw a ghost through the C API: one full stone structure imported and a marker placed at
// the player's feet (10, 64, -20), active.
pub struct GhostScene;

impl GhostScene {
    const DEADLINE: Duration = Duration::from_secs(60);
    pub const SIZE: [i32; 3] = [2, 2, 2];

    // A session in the world with the marker placed, ticked until a take holds jobs; that take is returned.
    pub fn start() -> (StSession, GhostSeen) {
        let session = StSession::new();
        Self::import(&session);
        Self::place(&session);
        let seen = Self::tick_until_jobs(&session, 1.0);
        (session, seen)
    }

    pub fn import(session: &StSession) {
        let path = session.data.path().join("tower.mcstructure");
        std::fs::write(&path, Self::stone(Self::SIZE)).unwrap();
        let text = path.to_string_lossy().into_owned();
        let view = StStr { ptr: text.as_ptr(), len: text.len() };
        assert_eq!(st_files_picked(session.handle, &view, 1), StStatus::Ok);
        let start = Instant::now();
        while session.shared().snapshot().unsorted.is_empty() {
            assert!(start.elapsed() < Self::DEADLINE, "the structure was not imported");
            assert_eq!(session.tick(&StSession::in_world()), StStatus::Ok);
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    pub fn place(session: &StSession) {
        let id = session.shared().snapshot().unsorted[0].id;
        let place = LibraryAction::PlaceMarker { name: "Home".to_string(), target: MarkerTarget::Structure(id) };
        session.shared().send(&mut vec![place]);
        assert_eq!(session.tick(&StSession::in_world()), StStatus::Ok);
    }

    // Ticks at `time` and takes after each tick until a take holds a job.
    pub fn tick_until_jobs(session: &StSession, time: f64) -> GhostSeen {
        let start = Instant::now();
        loop {
            assert_eq!(session.tick(&StSession::in_world_at(time)), StStatus::Ok);
            let seen = GhostSeen::take(session);
            if !seen.jobs.is_empty() {
                return seen;
            }
            assert!(start.elapsed() < Self::DEADLINE, "the ghost did not produce a job");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    // Answers every job of `seen` as built with `bytes`.
    pub fn report_built(session: &StSession, seen: &GhostSeen, bytes: u64) -> StStatus {
        let built: Vec<StSlotBuilt> = seen
            .jobs
            .iter()
            .map(|job| StSlotBuilt { slot: job.slot, serial: job.serial, generation: job.generation, bytes, ok: true })
            .collect();
        let report = StGhostReport {
            built: built.as_ptr(),
            built_count: built.len() as u32,
            unknown_blocks: 0,
            default_state_blocks: 0,
            palette_resolved: false,
            build_us: 0,
            draw_us_sum: 0,
            draw_frames: 0,
            draw_us_max: 0,
            meshes_drawn: 0,
            cells_built: 0,
            skipped_cells: 0,
            culled: 0,
            world_reset: false,
        };
        st_ghost_report(session.handle, &report)
    }

    // Answers the jobs of `seen` as built with `bytes`, then ticks, takes and answers again until a take holds no job.
    // Returns that take, which holds the draws of everything built.
    pub fn finish(session: &StSession, seen: &GhostSeen, bytes: u64) -> GhostSeen {
        let mut seen = seen.clone();
        for _ in 0..32 {
            assert_eq!(Self::report_built(session, &seen, bytes), StStatus::Ok);
            assert_eq!(session.tick(&StSession::in_world()), StStatus::Ok);
            seen = GhostSeen::take(session);
            if seen.jobs.is_empty() {
                return seen;
            }
        }
        panic!("the ghost kept handing out jobs");
    }

    // `.mcstructure` bytes of a box of `size` cells, every one stone.
    pub fn stone(size: [i32; 3]) -> Vec<u8> {
        let volume = size.iter().product::<i32>() as usize;
        let stone = NbtCompound::new(vec![
            ("name".to_string(), NbtTag::String("minecraft:stone".to_string())),
            ("states".to_string(), NbtTag::Compound(NbtCompound::default())),
            ("version".to_string(), NbtTag::Int(18_168_865)),
        ]);
        let default = NbtCompound::new(vec![
            ("block_palette".to_string(), NbtTag::List(vec![NbtTag::Compound(stone)])),
            ("block_position_data".to_string(), NbtTag::Compound(NbtCompound::default())),
        ]);
        let layers = NbtTag::List(vec![NbtTag::IntList(vec![0; volume]), NbtTag::IntList(vec![-1; volume])]);
        let palette = NbtCompound::new(vec![("default".to_string(), NbtTag::Compound(default))]);
        let structure = NbtCompound::new(vec![
            ("block_indices".to_string(), layers),
            ("entities".to_string(), NbtTag::List(Vec::new())),
            ("palette".to_string(), NbtTag::Compound(palette)),
        ]);
        NbtWriter::compound(
            "",
            &[
                ("format_version", NbtTag::Int(1)),
                ("size", NbtTag::IntList(size.to_vec())),
                ("structure", NbtTag::Compound(structure)),
                ("structure_world_origin", NbtTag::IntList(vec![0, 0, 0])),
            ],
        )
        .unwrap()
    }
}
