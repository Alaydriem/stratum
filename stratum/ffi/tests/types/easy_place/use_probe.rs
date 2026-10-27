use stratum_core::structs::easy_place::{Face, GameMode, SwapAnswer, UseProbe};
use stratum_ffi::types::StUseProbe;

fn probe() -> StUseProbe {
    StUseProbe {
        time_seconds: 2.0,
        pressed: true,
        held: true,
        eye: [1.0, 2.0, 3.0],
        dir: [0.0, -1.0, 0.0],
        reach: 5.0,
        game_mode: 1,
        has_hit: true,
        hit_cell: [4, 5, 6],
        hit_face: 5,
        hit_distance: 3.5,
        hit_replaceable: true,
        has_body: true,
        body_min: [1, 1, 3],
        body_max: [1, 2, 3],
        swap_answer: 2,
    }
}

#[test]
fn the_codes_read_in_the_documented_order() {
    let read = UseProbe::from(&probe());

    assert_eq!(read.game_mode, GameMode::Creative);
    assert_eq!(read.swap_answer, SwapAnswer::Refused);
    let hit = read.hit.unwrap();
    assert_eq!((hit.cell, hit.face, hit.distance, hit.replaceable), ([4, 5, 6], Face::East, 3.5, true));
}

#[test]
fn a_code_out_of_range_reads_as_the_safe_choice() {
    let read = UseProbe::from(&StUseProbe { game_mode: 9, hit_face: 6, swap_answer: 9, ..probe() });
    let no_hit = UseProbe::from(&StUseProbe { has_hit: false, ..probe() });

    assert_eq!(read.game_mode, GameMode::Other);
    assert_eq!(read.swap_answer, SwapAnswer::None);
    assert!(read.hit.is_none(), "a face past the last one is no hit");
    assert!(no_hit.hit.is_none());
}

#[test]
fn the_body_box_reads_only_when_set_and_ordered() {
    let read = UseProbe::from(&probe());
    let unset = UseProbe::from(&StUseProbe { has_body: false, ..probe() });
    let inverted = UseProbe::from(&StUseProbe { body_min: [1, 3, 3], ..probe() });

    assert_eq!(read.body, Some(([1, 1, 3], [1, 2, 3])));
    assert!(unset.body.is_none());
    assert!(inverted.body.is_none(), "a min above the max on one axis is no body");
}
