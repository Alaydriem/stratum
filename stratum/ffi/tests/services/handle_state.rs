use amm_client::test_support::FakeTable;
use amm_core::structs::descriptor::Capability;
use stratum_core::structs::easy_place::UseCommand;
use stratum_ffi::api::easy_place::{st_use_aim, st_use_decide};
use stratum_ffi::types::{StSpotWorld, StStatus, StUseCommand, StUseProbe};

use crate::support::{EasyPlace, StSession};

fn aim(session: &StSession) -> bool {
    let probe = EasyPlace::above(10, -20);
    let (mut spot, mut has_spot) = ([0; 3], false);
    assert_eq!(st_use_aim(session.handle, &probe, &mut spot, &mut has_spot), StStatus::Ok);
    has_spot
}

fn decide(session: &StSession) -> StUseCommand {
    let probe = EasyPlace::above(10, -20);
    let slots = EasyPlace::stone_in_hand();
    let world = StSpotWorld { against: [true; 6] };
    let mut out = StUseCommand::from(UseCommand::Block);
    assert_eq!(st_use_decide(session.handle, &probe, &world, slots.as_ptr(), 1, 0, &mut out), StStatus::Ok);
    out
}

#[test]
fn the_use_path_is_vanilla_while_the_library_is_open() {
    let session = EasyPlace::start();
    let probe = EasyPlace::above(10, -20);
    EasyPlace::aim_when_ready(&session, &probe);

    session.open();

    assert!(!aim(&session), "no spot while the library is open");
    assert_eq!(decide(&session).kind, 0);

    FakeTable::push_event(StSession::OPEN_KEY);
    session.tick(&StSession::in_world());

    EasyPlace::aim_when_ready(&session, &probe);
    assert_eq!(decide(&session).kind, 5);
}

#[test]
fn the_use_path_is_vanilla_while_another_mod_holds_input() {
    let session = EasyPlace::start();
    EasyPlace::aim_when_ready(&session, &EasyPlace::above(10, -20));

    FakeTable::set_held_by_other(Capability::Input, true);
    session.tick(&StSession::in_world());

    assert!(!aim(&session));
    assert_eq!(decide(&session).kind, 0);

    FakeTable::set_held_by_other(Capability::Input, false);
    session.tick(&StSession::in_world());

    assert!(aim(&session));
}

// Looks away from the build, then asks what to do with the use.
fn away_command(session: &StSession) -> StUseCommand {
    let probe = StUseProbe { dir: [0.0, 1.0, 0.0], ..EasyPlace::above(10, -20) };
    let slots = EasyPlace::stone_in_hand();
    let (mut spot, mut has_spot) = ([0; 3], false);
    assert_eq!(st_use_aim(session.handle, &probe, &mut spot, &mut has_spot), StStatus::Ok);
    assert!(!has_spot);
    let mut out = StUseCommand::from(UseCommand::Block);
    assert_eq!(st_use_decide(session.handle, &probe, std::ptr::null(), slots.as_ptr(), 1, 0, &mut out), StStatus::Ok);
    out
}

#[test]
fn closing_the_gate_ends_the_hold() {
    let session = EasyPlace::start();
    EasyPlace::aim_when_ready(&session, &EasyPlace::above(10, -20));
    assert_eq!(decide(&session).kind, 5);
    assert_eq!(away_command(&session).kind, 1, "a hold that placed blocks the rest of its uses");

    session.tick(&StSession::in_menu());
    session.tick(&StSession::in_world());

    assert_eq!(away_command(&session).kind, 0, "the hold ended when the gate closed");
}
