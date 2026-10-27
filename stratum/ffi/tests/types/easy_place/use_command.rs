use stratum_core::structs::easy_place::{Face, Placement, UseCommand};
use stratum_ffi::types::StUseCommand;

fn placement() -> Placement {
    Placement { spot: [1, 2, 3], neighbour: [1, 2, 2], face: Face::South, click: [0.5, 0.75, 1.0] }
}

#[test]
fn every_command_crosses_with_its_kind_and_only_its_fields() {
    let vanilla = StUseCommand::from(UseCommand::Vanilla);
    let select = StUseCommand::from(UseCommand::Select { slot: 4, place: placement() });
    let swap = StUseCommand::from(UseCommand::Swap { from: 20, to: 2 });
    let pick = StUseCommand::from(UseCommand::Pick { palette_id: 300, slot: 5 });
    let place = StUseCommand::from(UseCommand::Place(placement()));

    assert_eq!((vanilla.kind, vanilla.spot, vanilla.slot), (0, [0; 3], 0));
    assert_eq!((select.kind, select.slot, select.spot, select.neighbour, select.face), (2, 4, [1, 2, 3], [1, 2, 2], 3));
    assert_eq!((swap.kind, swap.from, swap.to, swap.spot, swap.slot), (3, 20, 2, [0; 3], 0));
    assert_eq!((pick.kind, pick.palette_id, pick.slot, pick.from, pick.neighbour), (4, 300, 5, 0, [0; 3]));
    assert_eq!((place.kind, place.slot, place.spot, place.face), (5, 0, [1, 2, 3], 3));
}

#[test]
fn the_click_point_crosses_the_boundary() {
    let select = StUseCommand::from(UseCommand::Select { slot: 4, place: placement() });
    let place = StUseCommand::from(UseCommand::Place(placement()));
    let swap = StUseCommand::from(UseCommand::Swap { from: 20, to: 2 });
    assert_eq!(select.click, [0.5, 0.75, 1.0]);
    assert_eq!(place.click, [0.5, 0.75, 1.0]);
    assert_eq!(swap.click, [0.0; 3]);
}
