use stratum_core::services::preview::TexturePack;
use stratum_core::structs::preview::{BlockTexture, TextureTint};

use crate::support::PreviewFixture;

const BLOCKS: &str = r#"{
  "format_version": [1, 1, 0],
  "oak_leaves": { // a comment, as the game's file has
    "textures": "oak_leaves"
  },
  "grass": { "textures": { "up": "grass_top", "down": "dirt", "side": "grass_side" } }
}"#;
const TERRAIN: &str = r##"{ "texture_data": {
  "oak_leaves": { "textures": ["textures/blocks/leaves_oak", "textures/blocks/leaves_oak_opaque"] },
  "grass_side": { "textures": [{ "path": "textures/blocks/grass_side", "overlay_color": "#79c05a" }] }
} }"##;

#[test]
fn a_pack_folder_gives_face_keys_and_tinted_textures() {
    let dir = tempfile::tempdir().unwrap();
    let grey = [200, 200, 200, 255];
    PreviewFixture::pack(
        dir.path(),
        BLOCKS,
        TERRAIN,
        &[("leaves_oak.tga", PreviewFixture::tga(1, &[vec![grey]])), ("stone.png", vec![0; 4])],
    );
    let mut pack = TexturePack::load(dir.path());
    assert_eq!(pack.pack_count(), 1);

    let keys = pack.face_keys("minecraft:oak_leaves").unwrap();
    let texture = pack.texture(&keys[0]).unwrap();
    let mut expected = grey.to_vec();
    TextureTint::Multiply(TextureTint::PLAINS_FOLIAGE).apply(&mut expected);
    assert_eq!(texture.rgba(), &expected[..]);

    // grass_block is listed as grass, with a key per face
    let grass = pack.face_keys("minecraft:grass_block").unwrap();
    assert_eq!(grass[0], "grass_top");
    assert_eq!(grass[1], "dirt");
    assert_eq!(grass[2], "grass_side");

    // a texture file named after the block counts when no list names the block
    assert!(pack.face_keys("minecraft:stone").is_some());
    assert!(pack.face_keys("minecraft:unknown_block").is_none());
}

#[test]
fn a_folder_with_no_packs_gives_an_empty_pack() {
    let dir = tempfile::tempdir().unwrap();
    let mut pack = TexturePack::load(dir.path());
    assert_eq!(pack.pack_count(), 0);
    assert!(pack.face_keys("minecraft:stone").is_none());
    assert!(pack.texture("stone").is_none());
}

#[test]
fn the_texture_cache_never_holds_more_than_its_capacity() {
    let mut pack = TexturePack::empty();
    let keys: [String; 6] = std::array::from_fn(|_| "k".to_owned());
    let textures: Vec<(String, BlockTexture)> = (0..TexturePack::CACHE_CAPACITY + 5)
        .map(|index| (format!("texture_{index}"), BlockTexture::solid([1, 2, 3, 255])))
        .collect();
    pack.insert("block", keys, textures);
    assert!(pack.cached() <= TexturePack::CACHE_CAPACITY);
    // the newest is kept: the cache empties before it would pass its capacity
    let newest = format!("texture_{}", TexturePack::CACHE_CAPACITY + 4);
    assert!(pack.texture(&newest).is_some());
    assert!(pack.texture("texture_0").is_none());
    assert!(pack.cached() <= TexturePack::CACHE_CAPACITY);
}
