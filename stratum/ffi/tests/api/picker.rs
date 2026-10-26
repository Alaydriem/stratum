use amm_abi::types::AmmLogLevel;
use stratum_core::config::ImportSettings;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::snapshot::ImportNotice;
use stratum_ffi::api::picker::{st_files_picked, st_take_picker_request};
use stratum_ffi::types::{PathList, StStatus, StStr};

use crate::support::StSession;

// Builds the picker answers the shim would send.
struct Picks;

impl Picks {
    fn view(text: &str) -> StStr {
        StStr { ptr: text.as_ptr(), len: text.len() }
    }

    fn pending(session: &StSession) -> usize {
        session.handle().with_state(|state| state.pending_imports()).unwrap()
    }
}

#[test]
fn picker_request_is_returned_once() {
    let session = StSession::new();
    session.shared().send(&mut vec![LibraryAction::RequestFilePicker(None)]);
    assert!(!st_take_picker_request(session.handle));

    assert_eq!(session.tick(&StSession::in_world()), StStatus::Ok);

    assert!(st_take_picker_request(session.handle));
    assert!(!st_take_picker_request(session.handle));
}

#[test]
fn null_handle_has_no_picker_request() {
    assert!(!st_take_picker_request(std::ptr::null_mut()));
}

#[test]
fn picked_paths_are_utf8_checked() {
    let session = StSession::new();
    let good = session.data.path().join("tower.mcstructure").to_string_lossy().into_owned();
    let bad = [0x66u8, 0xFF, 0x2E];
    let paths = [Picks::view(&good), StStr { ptr: bad.as_ptr(), len: bad.len() }];

    let status = st_files_picked(session.handle, paths.as_ptr(), 2);

    assert_eq!(status, StStatus::InvalidArgument);
    assert_eq!(Picks::pending(&session), 0);
}

#[test]
fn picked_paths_reach_the_import_worker() {
    let session = StSession::new();
    let first = session.data.path().join("a.mcstructure").to_string_lossy().into_owned();
    let second = session.data.path().join("b.mcstructure").to_string_lossy().into_owned();
    let paths = [Picks::view(&first), Picks::view(&second)];

    assert_eq!(st_files_picked(session.handle, paths.as_ptr(), 2), StStatus::Ok);

    assert_eq!(Picks::pending(&session), 2);
}

#[test]
fn picked_paths_past_the_bound_are_left_out_and_the_queue_refuses_the_rest() {
    let session = StSession::new();
    let names: Vec<String> = (0..=PathList::MAX)
        .map(|n| session.data.path().join(format!("{n}.mcstructure")).to_string_lossy().into_owned())
        .collect();
    let paths: Vec<StStr> = names.iter().map(|name| Picks::view(name)).collect();
    assert_eq!(paths.len(), PathList::MAX + 1);

    let status = st_files_picked(session.handle, paths.as_ptr(), paths.len() as u32);

    assert_eq!(status, StStatus::Ok);
    let lines = StSession::logged("picked files past the import queue were left out");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].level, AmmLogLevel::Warn as u8);
    // the worker takes the default queue of 64, plus at most a full result queue and one request in hand while
    // it runs, and refuses the rest, which the library shows as a full queue
    let queue = ImportSettings::default().queue;
    let accepted = Picks::pending(&session);
    assert!((queue..=2 * queue + 1).contains(&accepted), "accepted {accepted}");
    assert!(session.shared().snapshot().notices.iter().any(|notice| matches!(notice, ImportNotice::ImportQueueFull)));
}

#[test]
fn cancelled_picker_imports_nothing() {
    let session = StSession::new();

    assert_eq!(st_files_picked(session.handle, std::ptr::null(), 0), StStatus::Ok);

    assert_eq!(Picks::pending(&session), 0);
}

#[test]
fn null_paths_with_a_count_and_a_null_handle_are_invalid() {
    let session = StSession::new();
    let path = Picks::view(r"C:\a.mcstructure");

    assert_eq!(st_files_picked(session.handle, std::ptr::null(), 1), StStatus::InvalidArgument);
    assert_eq!(st_files_picked(std::ptr::null_mut(), &path, 1), StStatus::InvalidArgument);
}

#[test]
fn a_failed_import_is_logged_with_the_file_and_the_reason() {
    let session = StSession::new();
    let junk = session.data.path().join("junk.mcstructure");
    std::fs::write(&junk, b"not nbt").unwrap();
    let path = junk.to_string_lossy().into_owned();
    assert_eq!(st_files_picked(session.handle, &Picks::view(&path), 1), StStatus::Ok);

    for _ in 0..500 {
        assert_eq!(session.tick(&StSession::in_world()), StStatus::Ok);
        if Picks::pending(&session) == 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let lines = StSession::logged("structure import failed");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].level, AmmLogLevel::Warn as u8);
    assert!(lines[0].fields_json.contains("junk.mcstructure"));
    assert!(lines[0].fields_json.contains("reason"));
}
