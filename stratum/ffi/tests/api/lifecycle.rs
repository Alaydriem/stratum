use amm_abi::types::AmmLogLevel;
use amm_client::test_support::FakeTable;
use stratum_core::structs::action::LibraryAction;
use stratum_ffi::api::lifecycle::{st_create, st_destroy, st_log};
use stratum_ffi::types::{StStatus, StStr};

use crate::support::StSession;

#[test]
fn create_without_manager_returns_null() {
    let config = StSession::str(r"C:\mods\alaydriem-stratum\config");
    let data = StSession::str(r"C:\mods\alaydriem-stratum\data");

    assert!(st_create(&config, &data).is_null());
}

#[test]
fn null_or_non_utf8_folders_give_a_null_handle() {
    let data = StSession::str(r"C:\mods\alaydriem-stratum\data");
    let bad = [0xFFu8];
    let bad = StStr { ptr: bad.as_ptr(), len: 1 };

    assert!(st_create(std::ptr::null(), &data).is_null());
    assert!(st_create(&data, std::ptr::null()).is_null());
    assert!(st_create(&bad, &data).is_null());
}

#[test]
fn create_places_no_menu_entry() {
    let _session = StSession::new();

    let entries = FakeTable::with(|state| state.menu_entries.clone());

    assert!(entries.is_empty());
}

#[test]
fn destroy_saves_the_library_then_unregisters() {
    let mut session = StSession::new();
    session.shared().send(&mut vec![LibraryAction::CreateProject("Castle".to_owned())]);
    assert_eq!(session.tick(&StSession::in_world()), StStatus::Ok);

    session.destroy();

    let library = std::fs::read_to_string(session.data.path().join("library.hcl")).unwrap();
    assert!(library.contains("Castle"));
    assert_eq!(FakeTable::with(|state| state.unregistered.len()), 1);
}

#[test]
fn destroy_null_is_safe() {
    st_destroy(std::ptr::null_mut());
}

#[test]
fn shim_log_line_reaches_the_manager_with_its_level_and_fields() {
    let _session = StSession::new();
    let (message, fields, file) = ("shim line", "{\"count\":3}", "StratumMod.cpp");

    st_log(
        AmmLogLevel::Warn as u8,
        message.as_ptr(),
        message.len(),
        fields.as_ptr(),
        fields.len(),
        file.as_ptr(),
        file.len(),
        7,
    );

    let calls = StSession::logged("shim line");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].level, AmmLogLevel::Warn as u8);
    assert_eq!(calls[0].target, "alaydriem_stratum::shim");
    assert!(calls[0].fields_json.contains("\"count\":3"));
    assert_eq!((calls[0].file.as_str(), calls[0].line), ("StratumMod.cpp", 7));
}

#[test]
fn shim_log_line_with_a_null_text_is_dropped() {
    let _session = StSession::new();

    st_log(AmmLogLevel::Error as u8, std::ptr::null(), 4, std::ptr::null(), 0, std::ptr::null(), 0, 1);

    assert!(FakeTable::with(|state| state.logs.iter().all(|call| call.target != "alaydriem_stratum::shim")));
}
