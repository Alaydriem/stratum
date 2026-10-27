use stratum_core::services::preview::{PreviewMesher, PreviewRasterizer, TexturePack};
use stratum_core::structs::preview::{OrbitCamera, PreviewMesh};

use crate::support::{BlockFactory, PreviewFixture};

fn cube() -> PreviewMesh {
    let structure = PreviewFixture::structure([1, 1, 1], &[([0, 0, 0], BlockFactory::plain("stone"))]);
    PreviewMesher::build(&structure, &mut TexturePack::empty()).unwrap()
}

fn pixel(rasterizer: &PreviewRasterizer, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * rasterizer.width() + x) * 4) as usize;
    rasterizer.pixels()[at..at + 4].try_into().unwrap()
}

#[test]
fn a_cube_fills_the_middle_and_leaves_the_corners_clear() {
    let mut rasterizer = PreviewRasterizer::new();
    let drawn = rasterizer.render(&cube(), OrbitCamera::start(), 64, 48);
    // three faces turn towards the camera, two triangles each
    assert_eq!(drawn, 6);
    assert_eq!(pixel(&rasterizer, 32, 24)[3], 255);
    assert_eq!(pixel(&rasterizer, 0, 0), [0, 0, 0, 0]);
}

#[test]
fn the_top_face_is_brighter_than_the_sides() {
    let mut rasterizer = PreviewRasterizer::new();
    rasterizer.render(&cube(), OrbitCamera::new(45.0, 30.0), 200, 200);
    // the stand-in texture is one grey; the top shows just above the middle, the sides below it
    let top = pixel(&rasterizer, 100, 70);
    let side = pixel(&rasterizer, 100, 140);
    assert_eq!(top[3], 255);
    assert_eq!(side[3], 255);
    assert!(top[0] > side[0], "top {top:?} side {side:?}");
}

#[test]
fn the_image_is_capped_at_its_limits() {
    let mut rasterizer = PreviewRasterizer::new();
    rasterizer.render(&cube(), OrbitCamera::start(), 4000, 3000);
    assert_eq!(
        (rasterizer.width(), rasterizer.height()),
        (PreviewRasterizer::MAX_WIDTH, PreviewRasterizer::MAX_HEIGHT)
    );
    assert_eq!(rasterizer.pixels().len(), (PreviewRasterizer::MAX_WIDTH * PreviewRasterizer::MAX_HEIGHT * 4) as usize);
}

#[test]
fn a_render_clears_the_last_picture() {
    let mut rasterizer = PreviewRasterizer::new();
    rasterizer.render(&cube(), OrbitCamera::start(), 64, 48);
    let empty = PreviewMesh::new(cube().size, cube().textures);
    rasterizer.render(&empty, OrbitCamera::start(), 64, 48);
    assert!(rasterizer.pixels().iter().all(|byte| *byte == 0));
}

fn covered(rasterizer: &PreviewRasterizer) -> usize {
    rasterizer.pixels().as_chunks::<4>().0.iter().filter(|px| px[3] != 0).count()
}

#[test]
fn zoom_scales_the_picture_about_its_centre() {
    let mut rasterizer = PreviewRasterizer::new();
    rasterizer.render(&cube(), OrbitCamera::start(), 64, 64);
    let fitted = covered(&rasterizer);
    rasterizer.render(&cube(), OrbitCamera::start().zoomed(-5), 64, 64);
    let out = covered(&rasterizer);
    assert_eq!(pixel(&rasterizer, 32, 32)[3], 255);
    rasterizer.render(&cube(), OrbitCamera::start().zoomed(5), 64, 64);
    let close = covered(&rasterizer);
    assert!(out < fitted && fitted < close, "out {out} fitted {fitted} close {close}");
}
