use stratum_core::structs::easy_place::{Face, UseCommand};
use stratum_ffi::api::easy_place::{st_block_seen, st_use_aim, st_use_decide, st_use_released};
use stratum_ffi::services::StBoundary;
use stratum_ffi::types::{StInventorySlot, StSpotWorld, StStatus, StStr, StUseCommand};

use crate::support::{EasyPlace, StSession};

fn command() -> StUseCommand {
    StUseCommand::from(UseCommand::Vanilla)
}

#[test]
fn null_pointers_return_invalid_argument() {
    let session = StSession::new();
    let probe = EasyPlace::above(0, 0);
    let slots = EasyPlace::stone_in_hand();
    let (mut spot, mut has_spot, mut out) = ([0; 3], false, command());
    let name = StStr { ptr: EasyPlace::STONE.as_ptr(), len: EasyPlace::STONE.len() };
    let nameless = StStr { ptr: std::ptr::null(), len: 3 };

    assert_eq!(st_use_aim(std::ptr::null_mut(), &probe, &mut spot, &mut has_spot), StStatus::InvalidArgument);
    assert_eq!(st_use_aim(session.handle, std::ptr::null(), &mut spot, &mut has_spot), StStatus::InvalidArgument);
    assert_eq!(st_use_aim(session.handle, &probe, std::ptr::null_mut(), &mut has_spot), StStatus::InvalidArgument);
    assert_eq!(st_use_aim(session.handle, &probe, &mut spot, std::ptr::null_mut()), StStatus::InvalidArgument);

    let decide =
        |handle, probe, slots, count, out| st_use_decide(handle, probe, std::ptr::null(), slots, count, 0, out);
    assert_eq!(decide(std::ptr::null_mut(), &probe, slots.as_ptr(), 1, &mut out), StStatus::InvalidArgument);
    assert_eq!(decide(session.handle, std::ptr::null(), slots.as_ptr(), 1, &mut out), StStatus::InvalidArgument);
    assert_eq!(decide(session.handle, &probe, std::ptr::null(), 1, &mut out), StStatus::InvalidArgument);
    assert_eq!(decide(session.handle, &probe, slots.as_ptr(), 1, std::ptr::null_mut()), StStatus::InvalidArgument);
    let broken = [StInventorySlot { item: nameless, count: 1 }];
    assert_eq!(decide(session.handle, &probe, broken.as_ptr(), 1, &mut out), StStatus::InvalidArgument);

    assert_eq!(st_use_released(std::ptr::null_mut()), StStatus::InvalidArgument);
    assert_eq!(st_block_seen(std::ptr::null_mut(), 1, 2, 3, name, 1.0), StStatus::InvalidArgument);
    assert_eq!(st_block_seen(session.handle, 1, 2, 3, nameless, 1.0), StStatus::InvalidArgument);
}

#[test]
fn a_panic_does_not_cross_the_boundary() {
    let session = StSession::new();
    let probe = EasyPlace::above(0, 0);
    let (mut spot, mut has_spot, mut out) = ([0; 3], false, command());
    let name = StStr { ptr: EasyPlace::STONE.as_ptr(), len: EasyPlace::STONE.len() };

    let panicked = StBoundary::guard(|| {
        session.handle().with_state(|_| panic!("use"));
        StStatus::Ok
    });

    assert_eq!(panicked, StStatus::Panicked);
    assert_eq!(st_use_aim(session.handle, &probe, &mut spot, &mut has_spot), StStatus::Failed);
    let decided = st_use_decide(session.handle, &probe, std::ptr::null(), std::ptr::null(), 0, 0, &mut out);
    assert_eq!(decided, StStatus::Failed);
    assert_eq!(st_use_released(session.handle), StStatus::Failed);
    assert_eq!(st_block_seen(session.handle, 1, 2, 3, name, 1.0), StStatus::Failed);
    assert_eq!(StSession::logged("panic stopped at the C boundary").len(), 1);
}

#[test]
fn more_than_36_slots_is_invalid_argument() {
    let session = StSession::new();
    let probe = EasyPlace::above(0, 0);
    let slots = [EasyPlace::stone_in_hand()[0]; 37];
    let mut out = command();

    let at_limit = st_use_decide(session.handle, &probe, std::ptr::null(), slots.as_ptr(), 36, 0, &mut out);
    let over = st_use_decide(session.handle, &probe, std::ptr::null(), slots.as_ptr(), 37, 0, &mut out);

    assert_eq!(at_limit, StStatus::Ok);
    assert_eq!(over, StStatus::InvalidArgument);
}

#[test]
fn a_block_command_crosses_as_kind_one() {
    let session = EasyPlace::start();
    let probe = EasyPlace::above(10, -20);
    let spot = EasyPlace::aim_when_ready(&session, &probe);
    let slots = EasyPlace::stone_in_hand();
    let world = StSpotWorld { against: [true; 6] };
    let mut out = command();

    let status = st_use_decide(session.handle, &probe, &world, slots.as_ptr(), 1, 0, &mut out);

    assert_eq!(status, StStatus::Ok);
    assert_eq!(out.kind, 5, "the first use places");
    assert_eq!(out.spot, spot);
    let normal = Face::ALL[usize::from(out.face)].opposite().normal();
    assert_eq!(out.neighbour, [spot[0] + normal[0], spot[1] + normal[1], spot[2] + normal[2]]);

    let (mut again, mut has_spot) = ([0; 3], true);
    assert_eq!(st_use_aim(session.handle, &probe, &mut again, &mut has_spot), StStatus::Ok);
    assert!(!has_spot, "the placed spot is not offered again");
    let blocked = st_use_decide(session.handle, &probe, std::ptr::null(), slots.as_ptr(), 1, 0, &mut out);
    assert_eq!(blocked, StStatus::Ok);
    assert_eq!(out.kind, 1, "the rest of the hold is blocked");
    assert_eq!((out.slot, out.from, out.to, out.palette_id, out.spot, out.face), (0, 0, 0, 0, [0; 3], 0));

    assert_eq!(st_use_released(session.handle), StStatus::Ok);
}
