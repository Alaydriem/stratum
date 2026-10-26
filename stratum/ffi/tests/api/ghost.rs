use std::time::{Duration, Instant};

use amm_client::test_support::FakeTable;

use stratum_ffi::api::ghost::{st_block_changed, st_ghost_report, st_ghost_take};
use stratum_ffi::services::{ManagerSession, StBoundary};
use stratum_ffi::types::{StGhostFrame, StGhostJobMode, StGhostReport, StStatus};

use crate::support::{GhostScene, GhostSeen, Hud, StSession};

// Report values for the API tests.
struct Reports;

impl Reports {
    fn empty() -> StGhostReport {
        StGhostReport {
            built: std::ptr::null(),
            built_count: 0,
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
        }
    }
}

#[test]
fn null_handle_or_out_is_invalid_argument() {
    let session = StSession::new();
    let mut out = StGhostFrame::empty();
    let report = Reports::empty();
    let missing_entries = StGhostReport { built_count: 1, ..Reports::empty() };

    assert_eq!(st_ghost_take(std::ptr::null_mut(), &mut out), StStatus::InvalidArgument);
    assert_eq!(st_ghost_take(session.handle, std::ptr::null_mut()), StStatus::InvalidArgument);
    assert_eq!(st_ghost_report(std::ptr::null_mut(), &report), StStatus::InvalidArgument);
    assert_eq!(st_ghost_report(session.handle, std::ptr::null()), StStatus::InvalidArgument);
    assert_eq!(st_block_changed(std::ptr::null_mut(), 1, 2, 3), StStatus::InvalidArgument);
    // a built count with no entries is refused once the count is read at all
    session.tick(&StSession::in_world());
    assert_eq!(st_ghost_report(session.handle, &missing_entries), StStatus::InvalidArgument);
}

#[test]
fn panic_does_not_cross_the_boundary() {
    let session = StSession::new();
    let ghost = session.handle().ghost().clone();

    let take = StBoundary::guard(|| {
        let _take = ghost.take();
        panic!("take");
    });
    let report = StBoundary::guard(|| {
        ghost.report(false, |_| panic!("report"));
        StStatus::Ok
    });
    let changed = StBoundary::guard(|| {
        session.handle().with_state(|_| panic!("block changed"));
        StStatus::Ok
    });

    assert_eq!((take, report, changed), (StStatus::Panicked, StStatus::Panicked, StStatus::Panicked));
    let mut out = StGhostFrame::empty();
    assert_eq!(st_ghost_take(session.handle, &mut out), StStatus::Failed);
    assert_eq!(st_ghost_report(session.handle, &Reports::empty()), StStatus::Failed);
    assert_eq!(st_block_changed(session.handle, 1, 2, 3), StStatus::Failed);
    assert_eq!(StSession::logged("panic stopped at the C boundary").len(), 3);
}

#[test]
fn block_changed_reaches_the_controller_on_the_next_tick() {
    let (session, seen) = GhostScene::start();
    let slot = seen.full_job().slot;
    GhostScene::finish(&session, &seen, 20 * 24);

    assert_eq!(st_block_changed(session.handle, 10, 64, -20), StStatus::Ok);
    assert!(GhostSeen::take(&session).jobs.is_empty());
    session.tick(&StSession::in_world());

    let rebuilt = GhostSeen::take(&session);
    assert_eq!(rebuilt.jobs.len(), 1);
    assert!(rebuilt.full_job().skip_matching);
    assert_eq!(rebuilt.full_job().slot, slot);
}

#[test]
fn a_second_take_before_the_tick_gets_the_draws_and_no_jobs() {
    let (session, seen) = GhostScene::start();
    let first = GhostScene::finish(&session, &seen, 480);
    assert_eq!(first.unscaled_draws().len(), 1);
    assert_eq!(first.miniature_draws().len(), 1);

    let second = GhostSeen::take(&session);

    assert_eq!(second.status, StStatus::Ok);
    assert!(second.jobs.is_empty() && second.frees.is_empty() && !second.clear_all && second.palette.is_none());
    assert_eq!(second.draws, first.draws);
}

#[test]
fn a_job_carries_the_padded_grid_of_its_section() {
    let (_session, seen) = GhostScene::start();

    let job = *seen.full_job();
    // the marker at (10, 64, -20) lies in section (0, 4, -2); a new marker shows the whole build, so the job is
    // the whole section
    assert_eq!(job.origin, [0, 64, -32]);
    assert_eq!(job.dims, [16, 16, 16]);
    assert!(job.skip_matching);
    assert_eq!(job.generation, seen.generation);
    let index = seen.jobs.iter().position(|candidate| candidate.skip_matching).unwrap();
    let cells = &seen.job_cells[index];
    assert_eq!(cells.len(), 18 * 18 * 18);
    // the padded grid starts one cell below the origin, X fastest, then Z, then Y
    let at = |world: [i32; 3]| {
        let [x, y, z] = [0, 1, 2].map(|axis| (world[axis] - job.origin[axis] + 1) as usize);
        cells[(y * 18 + z) * 18 + x]
    };
    assert_eq!(at([10, 64, -20]), 0);
    assert_eq!(at([11, 64, -19]), 0);
    assert_eq!(at([12, 64, -20]), u16::MAX);
    assert_eq!(at([-1, 63, -33]), u16::MAX);
}

#[test]
fn the_take_carries_the_palette_and_opacity() {
    let session = StSession::new();
    GhostScene::import(&session);
    GhostScene::place(&session);
    let start = Instant::now();
    let mut palette = None;
    let mut opacity = 0.0;
    while palette.is_none() {
        assert!(start.elapsed() < Duration::from_secs(60), "no palette arrived");
        session.tick(&StSession::in_world());
        let seen = GhostSeen::take(&session);
        palette = palette.or(seen.palette);
        opacity = seen.opacity;
        std::thread::sleep(Duration::from_millis(2));
    }

    let palette = palette.unwrap();
    assert_eq!(palette.len(), 2);
    // a little-endian root compound that names the block
    assert_eq!(palette[0][0], 10);
    assert!(palette[0].windows(b"minecraft:stone".len()).any(|window| window == b"minecraft:stone"));
    assert!((opacity - 0.5).abs() < 1e-6);
}

#[test]
fn a_built_report_turns_into_a_draw_at_the_job_origin() {
    let (session, seen) = GhostScene::start();
    assert!(seen.draws.is_empty());
    let job = *seen.full_job();
    assert_eq!(job.mode, StGhostJobMode::Replace);
    assert_eq!(job.slot_size, job.dims);

    // the mini ghost is one Append for its only section, placed relative to the build's lowest corner
    // (10, 64, -20) because the section starts at (0, 64, -32), and then one Finish
    let appends: Vec<_> = seen.jobs.iter().filter(|candidate| candidate.mode == StGhostJobMode::Append).collect();
    assert_eq!(appends.len(), 1);
    assert_eq!(appends[0].group_index, 0);
    assert_ne!(appends[0].group_serial, 0);
    assert_eq!(appends[0].group_offset, [-10, 0, -12]);
    assert_eq!(appends[0].origin, [10, 64, -20]);
    assert_eq!(appends[0].slot_size, [2, 2, 2]);
    assert!(seen.jobs.iter().all(|candidate| candidate.mode != StGhostJobMode::Finish));

    assert_eq!(GhostScene::report_built(&session, &seen, 480), StStatus::Ok);
    session.tick(&StSession::in_world());
    let next = GhostSeen::take(&session);
    assert_eq!(next.jobs.len(), 1);
    assert_eq!(next.jobs[0].mode, StGhostJobMode::Finish);
    assert_eq!(next.jobs[0].group_serial, appends[0].group_serial);
    assert_eq!(next.jobs[0].group_index, 1);
    assert_eq!(next.jobs[0].slot, appends[0].slot);

    let drawn = GhostScene::finish(&session, &next, 480);
    let ghost = drawn.unscaled_draws();
    assert_eq!(ghost.len(), 1);
    assert_eq!((ghost[0].slot, ghost[0].origin, ghost[0].offset), (job.slot, job.origin, [0, 0, 0]));
    // the mini ghost scales about the build's lowest corner and ends on it: the 2-block build moves it 2 cells out on
    // X and Z
    let mini = drawn.miniature_draws();
    assert_eq!(mini.len(), 1);
    assert_eq!((mini[0].origin, mini[0].offset, mini[0].scale), ([10, 64, -20], [-2, 0, -2], 0.25));
}

// The render thread draws many frames per tick: the build answer comes in the first report, and the empty reports
// of the frames after it fill the queue past its capacity before the tick drains it.
#[test]
fn a_build_answer_followed_by_twenty_reports_is_drawn_after_the_drain() {
    let (session, seen) = GhostScene::start();
    let slot = seen.full_job().slot;
    assert_eq!(GhostScene::report_built(&session, &seen, 480), StStatus::Ok);
    for _ in 0..20 {
        assert_eq!(st_ghost_report(session.handle, &Reports::empty()), StStatus::Ok);
    }

    session.tick(&StSession::in_world());

    let drawn = GhostSeen::take(&session);
    let ghost = drawn.unscaled_draws();
    assert_eq!(ghost.len(), 1);
    assert_eq!(ghost[0].slot, slot);
    assert_eq!(drawn.jobs.len(), 1);
    assert_eq!(drawn.jobs[0].mode, StGhostJobMode::Finish);
    let done = GhostScene::finish(&session, &drawn, 480);
    assert_eq!(done.miniature_draws().len(), 1);
}

#[test]
fn the_take_carries_the_world_epoch_which_moves_when_the_world_is_left() {
    let (session, seen) = GhostScene::start();
    let epoch = seen.world_epoch;
    session.tick(&StSession::in_world());
    let inside = GhostSeen::take(&session);
    assert_eq!(inside.world_epoch, epoch);
    assert!(inside.in_world);

    session.tick(&StSession::out_of_world());
    let outside = GhostSeen::take(&session);
    assert!(!outside.in_world);
    session.tick(&StSession::in_world());

    let back = GhostSeen::take(&session);
    assert_ne!(back.world_epoch, epoch);
    assert_ne!(back.world_epoch, outside.world_epoch);
    assert!(back.in_world);
}

#[test]
fn a_world_reset_report_brings_a_new_palette_and_a_clear() {
    let (session, seen) = GhostScene::start();
    GhostScene::report_built(&session, &seen, 480);
    session.tick(&StSession::in_world());
    let built = GhostSeen::take(&session);
    assert!(built.palette.is_none() && !built.clear_all);

    let reset = StGhostReport { world_reset: true, ..Reports::empty() };
    assert_eq!(st_ghost_report(session.handle, &reset), StStatus::Ok);
    session.tick(&StSession::in_world());

    let fresh = GhostSeen::take(&session);
    assert!(fresh.clear_all);
    assert_eq!(fresh.palette.map(|entries| entries.len()), Some(2));
    assert_ne!(fresh.generation, seen.generation);
    assert!(!fresh.jobs.is_empty());
}

#[test]
fn the_take_carries_the_rotation_of_a_turned_marker() {
    let (session, seen) = GhostScene::start();
    assert_eq!(seen.rotation, 0);

    // Full mode, Left from Layer wraps to Rotate, and Up turns the marker a quarter turn clockwise
    FakeTable::push_event(Hud::press(ManagerSession::HUD_MODE_ACTION));
    FakeTable::push_event(Hud::press(ManagerSession::HUD_LEFT_ACTION));
    FakeTable::push_event(Hud::press(ManagerSession::HUD_UP_ACTION));
    let start = Instant::now();
    loop {
        assert!(start.elapsed() < Duration::from_secs(60), "the turn did not reach the take");
        session.tick(&StSession::in_world());
        if GhostSeen::take(&session).rotation == 1 {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
