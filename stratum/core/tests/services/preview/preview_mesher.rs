use stratum_core::errors::PreviewError;
use stratum_core::services::preview::{PreviewMesher, TexturePack};
use stratum_core::structs::preview::PreviewMesh;
use stratum_core::structs::structure::{StateValue, StructureSize};

use crate::support::{BlockFactory, PreviewFixture};

fn mesh(size: [u32; 3], blocks: &[([u32; 3], &str)]) -> PreviewMesh {
    let blocks: Vec<_> = blocks.iter().map(|(at, name)| (*at, BlockFactory::plain(name))).collect();
    PreviewMesher::build(&PreviewFixture::structure(size, &blocks), &mut TexturePack::empty()).unwrap()
}

#[test]
fn two_touching_stone_cubes_hide_the_faces_between_them() {
    let mesh = mesh([2, 1, 1], &[([0, 0, 0], "stone"), ([1, 0, 0], "stone")]);
    assert_eq!(mesh.quads.len(), 10);
}

#[test]
fn air_and_structure_void_draw_nothing_and_hide_nothing() {
    let mesh = mesh([3, 1, 1], &[([0, 0, 0], "stone"), ([1, 0, 0], "air"), ([2, 0, 0], "structure_void")]);
    assert_eq!(mesh.quads.len(), 6);
}

#[test]
fn leaves_keep_the_faces_between_them_and_glass_does_not() {
    let leaves = mesh([2, 1, 1], &[([0, 0, 0], "oak_leaves"), ([1, 0, 0], "oak_leaves")]);
    assert_eq!(leaves.quads.len(), 12);
    let glass = mesh([2, 1, 1], &[([0, 0, 0], "glass"), ([1, 0, 0], "glass")]);
    assert_eq!(glass.quads.len(), 10);
    // stone beside glass keeps its face: glass hides only glass
    let mixed = mesh([2, 1, 1], &[([0, 0, 0], "stone"), ([1, 0, 0], "glass")]);
    assert_eq!(mixed.quads.len(), 11);
}

#[test]
fn a_bottom_slab_under_stone_keeps_its_top_face() {
    let slab = BlockFactory::block("oak_slab", &[("minecraft:vertical_half", StateValue::Text("bottom".into()))]);
    let structure =
        PreviewFixture::structure([1, 2, 1], &[([0, 0, 0], slab), ([0, 1, 0], BlockFactory::plain("stone"))]);
    let mesh = PreviewMesher::build(&structure, &mut TexturePack::empty()).unwrap();
    // the slab's top is at half height, so the stone above does not touch it; the stone's bottom shows too
    assert_eq!(mesh.quads.len(), 12);
    let top = mesh.quads.iter().filter(|quad| quad.normal == [0, 1, 0]).map(|quad| quad.corners[0][1]).min();
    assert_eq!(top, Some(8));
}

#[test]
fn stairs_draw_a_slab_and_a_step_on_the_high_side() {
    let stairs = BlockFactory::block(
        "oak_stairs",
        &[("upside_down_bit", StateValue::Int(0)), ("weirdo_direction", StateValue::Int(0))],
    );
    let mesh =
        PreviewMesher::build(&PreviewFixture::structure([1, 1, 1], &[([0, 0, 0], stairs)]), &mut TexturePack::empty())
            .unwrap();
    // six slab faces and five step faces: the step's bottom sits on the slab
    assert_eq!(mesh.quads.len(), 11);
    let step_top = mesh.quads.iter().find(|quad| quad.normal == [0, 1, 0] && quad.corners[0][1] == 16).unwrap();
    assert!(step_top.corners.iter().all(|corner| corner[0] >= 8));
}

#[test]
fn a_cross_plant_is_two_quads_seen_from_both_sides() {
    let mesh = mesh([1, 1, 1], &[([0, 0, 0], "short_grass")]);
    assert_eq!(mesh.quads.len(), 2);
    assert!(mesh.quads.iter().all(|quad| quad.normal == [0, 0, 0]));
}

#[test]
fn quads_sit_at_their_cell_in_sixteenths() {
    let mesh = mesh([3, 2, 2], &[([2, 1, 1], "stone")]);
    let lowest = mesh.quads.iter().flat_map(|quad| quad.corners).min().unwrap();
    assert_eq!(lowest, [32, 16, 16]);
}

#[test]
fn blocks_with_no_texture_use_the_stand_in_and_are_counted() {
    let mesh = mesh([2, 1, 1], &[([0, 0, 0], "stone"), ([1, 0, 0], "dirt")]);
    assert_eq!(mesh.missing, 2);
    assert!(mesh.quads.iter().all(|quad| quad.texture == 0));
}

#[test]
fn a_side_past_the_corner_range_is_refused() {
    let mut structure = PreviewFixture::structure([1, 1, 1], &[([0, 0, 0], BlockFactory::plain("stone"))]);
    structure.size = StructureSize::new(PreviewMesher::MAX_SIDE + 1, 1, 1);
    let result = PreviewMesher::build(&structure, &mut TexturePack::empty());
    assert_eq!(result, Err(PreviewError::TooLarge));
}
