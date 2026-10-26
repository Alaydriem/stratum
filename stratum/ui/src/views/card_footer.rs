use egui::{Frame, Ui};
use levi_ui::draw::Draw;
use levi_ui::tokens::{ColorToken, SpacingToken, color, spacing};

// The footer of a card: a divider, then a row of buttons pinned to the card's bottom edge. A card with more content than
// room keeps the footer under it.
pub struct CardFooter;

impl CardFooter {
    // The room between the card's outer edge and its content, at the bottom.
    pub fn inset(frame: &Frame) -> f32 {
        f32::from(frame.inner_margin.bottom) + frame.stroke.width
    }

    // `card_bottom` is the y of the card's outer bottom edge and `inset` the room `CardFooter::inset` gave. Call it
    // inside the card's frame, after the card's other content.
    pub fn show(ui: &mut Ui, card_bottom: f32, inset: f32, content: impl FnOnce(&mut Ui)) {
        let height = <spacing::Button as SpacingToken>::VALUE;
        let gap = <spacing::L as SpacingToken>::VALUE;
        let least = <spacing::M as SpacingToken>::VALUE;
        let divider = (card_bottom - inset - height - gap).max(ui.cursor().top() + least);
        let sides = ui.min_rect().x_range();
        let edge = sides.min - inset..=sides.max + inset;
        Draw::divider(ui.painter(), edge.into(), divider, <color::Line as ColorToken>::VALUE);
        ui.add_space(divider + gap - ui.cursor().top());
        content(ui);
    }
}
