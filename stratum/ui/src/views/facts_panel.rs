use egui::Ui;
use levi_i18n::text::{Args, Text, UserText};
use levi_ui::tokens::rule::Elevation;
use levi_ui::tokens::{SpacingToken, spacing};
use levi_ui::widget::{SectionHeader, Surface, SurfaceLevel, ValueRow};
use stratum_core::structs::marker::MarkerTarget;
use stratum_core::structs::snapshot::{LibrarySnapshot, StructureRow};

use super::{CardFooter, ItemActions, MarkerActions, SizeBox, StructurePreview};
use crate::message::StratumMessage;
use crate::presenter::DigitGroups;
use crate::state::{DialogItem, LibraryUiState};

// The card of one structure: its size box, then project, size, layers, blocks and whether the file carries a saved
// position, its actions, and a footer with the marker buttons pinned to the card's bottom.
pub struct FactsPanel;

impl FactsPanel {
    // `project` is the name of the structure's project, or None for an unsorted structure. `height` is the room the card fills
    // from where it starts, so the footer sits at the bottom of that room.
    pub fn show(
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        row: &StructureRow,
        project: Option<&str>,
        state: &mut LibraryUiState,
        height: f32,
        preview: Option<&mut StructurePreview>,
    ) {
        let bottom = ui.cursor().top() + height;
        let frame = Surface::frame(SurfaceLevel::Raised, Elevation::Flat);
        let inset = CardFooter::inset(&frame);
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            match preview {
                Some(preview) => preview.show(ui, row),
                None => SizeBox::show(ui, row.size),
            }
            ui.add_space(<spacing::M as SpacingToken>::VALUE);
            SectionHeader::new(Text::message(StratumMessage::StructureFacts)).divider(false).show(ui);
            let project = match project {
                Some(name) => Text::user(UserText::from_model(name.to_owned())),
                None => Text::message(StratumMessage::Unsorted),
            };
            let size = Args::new()
                .uint("x", u64::from(row.size.x))
                .uint("y", u64::from(row.size.y))
                .uint("z", u64::from(row.size.z));
            let origin =
                if row.has_origin { StratumMessage::SavedPositionKept } else { StratumMessage::NoSavedPosition };
            let facts = [
                (StratumMessage::FactProject, project),
                (StratumMessage::FactSize, Text::format(StratumMessage::SizeDims, size)),
                (StratumMessage::FactLayers, Text::user(UserText::from_model(DigitGroups::of(u64::from(row.size.y))))),
                (StratumMessage::FactBlocks, Text::user(UserText::from_model(DigitGroups::of(row.block_count)))),
                (StratumMessage::FactSavedOrigin, Text::message(origin)),
            ];
            for (label, value) in facts {
                ValueRow::new(Text::message(label), value).show(ui);
            }
            let gap = <spacing::M as SpacingToken>::VALUE;
            ui.add_space(gap);
            ItemActions::show(ui, snapshot, DialogItem::Structure(row.id), Some(row), state);
            CardFooter::show(ui, bottom, inset, |ui| {
                MarkerActions::show(ui, snapshot, MarkerTarget::Structure(row.id), None, state);
            });
        });
    }
}
