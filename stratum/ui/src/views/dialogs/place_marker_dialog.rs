use egui::{Context, Id};
use levi_i18n::text::{Args, Text, UserText};
use levi_ui::widget::{Caption, TextField, TextFieldKind};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::marker::MarkerTarget;
use stratum_core::structs::snapshot::LibrarySnapshot;

use super::{DialogForm, DialogOutcome, FormAnswer};
use crate::message::StratumMessage;
use crate::presenter::{ItemNames, NameCheck};
use crate::views::StratumId;

// The form that places a marker at the player's feet. It names the marker; the name starts as the target's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceMarkerDialog {
    target: MarkerTarget,
    // None until the first frame reads the target's name from the snapshot.
    name: Option<String>,
}

impl PlaceMarkerDialog {
    pub fn new(target: MarkerTarget) -> Self {
        Self { target, name: None }
    }

    pub fn target(&self) -> MarkerTarget {
        self.target
    }

    pub fn show(&mut self, ctx: &Context, snapshot: &LibrarySnapshot, out: &mut Vec<LibraryAction>) -> DialogOutcome {
        let Some(target_name) = ItemNames::target(snapshot, self.target) else {
            return DialogOutcome::Closed;
        };
        let target_name = target_name.to_owned();
        let name = self.name.get_or_insert_with(|| target_name.clone());
        let blocked = if !snapshot.in_world {
            Some(Text::message(StratumMessage::NotInWorldReason))
        } else if snapshot.markers_read_only {
            Some(Text::message(StratumMessage::MarkersReadOnlyTitle))
        } else {
            NameCheck::reason(name)
        };
        let shows = Text::format(
            StratumMessage::PlaceMarkerTarget,
            Args::new().user("target", UserText::from_model(target_name)),
        );
        let form = DialogForm::new(
            Text::message(StratumMessage::PlaceMarkerHere),
            Text::message(StratumMessage::PlaceMarkerHere),
        );
        let answer = form.blocked(blocked).show(ctx, |ui| {
            let placeholder = Text::message(StratumMessage::FactName);
            let width = ui.available_width();
            TextField::new(Id::unique(StratumId::DialogField), name, TextFieldKind::Plain, placeholder).show(ui, width);
            Caption::new(shows).show(ui);
        });
        match answer {
            FormAnswer::Pending => DialogOutcome::Open,
            FormAnswer::Cancel => DialogOutcome::Closed,
            FormAnswer::Confirm => {
                let name = self.name.as_deref().unwrap_or_default().trim().to_owned();
                if NameCheck::reason(&name).is_some() || !snapshot.in_world || snapshot.markers_read_only {
                    return DialogOutcome::Open;
                }
                out.push(LibraryAction::PlaceMarker { name, target: self.target });
                DialogOutcome::Closed
            }
        }
    }
}
