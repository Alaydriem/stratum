use stratum_core::structs::library::StructureId;
use stratum_ui::state::Page;

use crate::support::{ShapeProbe, SnapshotFixture, ViewSession};

#[test]
fn the_structure_page_lists_no_name_row_and_draws_the_size_box() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    session.state.page = Page::Structure(StructureId(1));
    session.frame(Vec::new());
    assert!(ShapeProbe::text_rect(&session.shapes, "STRUCTURE FACTS").is_some());
    assert!(ShapeProbe::exact_rects(&session.shapes, "Name").is_empty());
    // fixture structure is 32 x 18 x 24: the X and Z lengths and the Y length with the Layers fact
    assert!(!ShapeProbe::exact_rects(&session.shapes, "32").is_empty());
    assert!(!ShapeProbe::exact_rects(&session.shapes, "24").is_empty());
    assert_eq!(ShapeProbe::exact_rects(&session.shapes, "18").len(), 2);
}
