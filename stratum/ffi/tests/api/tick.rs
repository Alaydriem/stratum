use amm_client::test_support::FakeTable;
use amm_core::structs::descriptor::Capability;
use stratum_ffi::api::tick::st_tick;
use stratum_ffi::services::StBoundary;
use stratum_ffi::types::{StFrameStatus, StStatus, StStr};

use crate::support::StSession;

#[test]
fn null_handle_and_null_status_return_invalid_argument() {
    let session = StSession::new();
    let status = StSession::in_world();

    assert_eq!(st_tick(std::ptr::null_mut(), &status), StStatus::InvalidArgument);
    assert_eq!(st_tick(session.handle, std::ptr::null()), StStatus::InvalidArgument);
}

#[test]
fn a_non_utf8_world_name_still_ticks_in_that_world() {
    let session = StSession::new();
    let bad = [0xC3u8, 0x28];
    let status = StFrameStatus { world: StStr { ptr: bad.as_ptr(), len: bad.len() }, ..StSession::in_world() };

    assert_eq!(session.tick(&status), StStatus::Ok);
    assert_eq!(session.tick(&status), StStatus::Ok);
    assert!(session.shared().snapshot().in_world);
}

#[test]
fn a_null_world_name_with_a_length_returns_invalid_argument() {
    let session = StSession::new();
    let status = StFrameStatus { world: StStr { ptr: std::ptr::null(), len: 4 }, ..StSession::in_world() };

    assert_eq!(session.tick(&status), StStatus::InvalidArgument);
}

#[test]
fn tick_in_and_out_of_world_is_ok() {
    let session = StSession::new();

    assert_eq!(session.tick(&StSession::in_world()), StStatus::Ok);
    assert_eq!(session.tick(&StSession::out_of_world()), StStatus::Ok);
    assert!(session.shared().snapshot().revision > 0);
}

#[test]
fn panic_under_the_tick_lock_gives_panicked_then_failed_and_gives_the_ui_back() {
    let session = StSession::new();
    session.open();

    let status = StBoundary::guard(|| {
        session.handle().with_state(|_| panic!("tick"));
        StStatus::Ok
    });

    assert_eq!(status, StStatus::Panicked);
    // the lock the panic poisoned turns later ticks into Failed, logged once
    assert_eq!(session.tick(&StSession::in_world()), StStatus::Failed);
    assert_eq!(session.tick(&StSession::in_world()), StStatus::Failed);
    assert_eq!(StSession::logged("panic stopped at the C boundary").len(), 1);
    assert_eq!(StSession::logged("poisoned").len(), 1);
    // the first call that finds the lock poisoned closes the library and gives its leases back
    assert!(!session.shared().is_shown());
    assert_eq!(FakeTable::with(|state| state.released.clone()), [Capability::Input, Capability::Overlay]);
}
