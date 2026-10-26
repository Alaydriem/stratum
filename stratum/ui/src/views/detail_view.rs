use egui::Ui;
use levi_i18n::text::Text;
use levi_ui::icon::Icon;
use levi_ui::tokens::{SpacingToken, spacing};
use levi_ui::widget::{Banner, BannerTone, ButtonKind, EmptyState};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::snapshot::LibrarySnapshot;

use super::{ImportButton, ImportNotices, MarkerActions, ProjectPage, StructureListPage, StructurePage};
use crate::message::StratumMessage;
use crate::state::{LibraryUiState, Page};

// The detail column: the read-only notices, then the empty library or the page for the open selection. A project
// page keeps its own empty state in a library with no structures.
pub struct DetailView {
    project_page: ProjectPage,
    structure_page: StructurePage,
}

impl DetailView {
    pub fn new() -> Self {
        Self { project_page: ProjectPage::new(), structure_page: StructurePage::new() }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        state: &mut LibraryUiState,
        out: &mut Vec<LibraryAction>,
    ) {
        let gap = <spacing::M as SpacingToken>::VALUE;
        ImportNotices::show(ui, snapshot);
        MarkerActions::show_missing(ui, snapshot, state);
        if snapshot.library_read_only {
            Banner::new(BannerTone::Warning, Text::message(StratumMessage::LibraryReadOnlyTitle))
                .body(Text::message(StratumMessage::LibraryReadOnlyBody))
                .show(ui);
            ui.add_space(gap);
        }
        if snapshot.markers_read_only {
            Banner::new(BannerTone::Warning, Text::message(StratumMessage::MarkersReadOnlyTitle))
                .body(Text::message(StratumMessage::MarkersReadOnlyBody))
                .show(ui);
            ui.add_space(gap);
        }
        if snapshot.all_count == 0 && !matches!(state.page, Page::Project(_)) {
            let empty = EmptyState::new(
                Icon::Folder,
                Text::message(StratumMessage::EmptyTitle),
                Text::message(StratumMessage::EmptyMessage),
            )
            .action(ImportButton::build(snapshot, ButtonKind::Primary));
            if empty.show(ui).is_some_and(|action| action.clicked()) {
                out.push(LibraryAction::RequestFilePicker(ImportButton::destination(state.page)));
            }
            return;
        }
        match state.page {
            Page::Project(id) => {
                if let Some(project) = snapshot.projects.iter().find(|project| project.id == id) {
                    self.project_page.show(ui, snapshot, project, state, out);
                }
            }
            Page::Structure(id) => {
                let rows = snapshot
                    .unsorted
                    .iter()
                    .chain(snapshot.projects.iter().flat_map(|project| project.structures.iter()));
                if let Some(row) = rows.into_iter().find(|row| row.id == id) {
                    self.structure_page.show(ui, snapshot, row, state, out);
                }
            }
            Page::All | Page::Unsorted => StructureListPage::show(ui, snapshot, state.page, state),
        }
        // leaving a materials tab lets the next visit ask for its list again
        if state.materials_scope().is_none() {
            state.materials_requested = None;
            state.materials_shown = None;
            state.materials_reading = None;
            state.materials_failed = None;
        }
    }
}

impl Default for DetailView {
    fn default() -> Self {
        Self::new()
    }
}
