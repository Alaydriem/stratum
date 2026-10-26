use levi_rules::source::SourceFiles;
use levi_rules::widget_name::{DuplicateScan, WidgetNames};

#[test]
fn no_duplicate_widgets() {
    let names = WidgetNames::levi_ui().unwrap_or_else(|error| panic!("{error}"));
    let files = SourceFiles::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    let hits = DuplicateScan::scan(&names, &files).unwrap_or_else(|error| panic!("{error}"));
    assert!(
        hits.is_empty(),
        "copies of levi-ui widgets:\n{}",
        hits.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")
    );
}
