use std::sync::Arc;

use egui::{Align2, ColorImage, Event, Rect, Sense, TextureHandle, TextureOptions, Ui, Vec2, pos2, vec2};
use levi_i18n::text::{Args, Text, UserText};
use levi_ui::draw::{Corner, Draw, TextFit, TextStyle, TypeSize};
use levi_ui::font::FontFace;
use levi_ui::tokens::{ColorToken, color};
use stratum_core::services::preview::{PreviewRasterizer, PreviewService};
use stratum_core::structs::library::StructureId;
use stratum_core::structs::preview::{OrbitCamera, PreviewImage, PreviewRequest};
use stratum_core::structs::snapshot::StructureRow;

use crate::message::StratumMessage;

// The 3D preview of one structure: the picture the preview thread drew, turned by dragging in the pane. A new
// picture is asked for only when the structure, the turn, the zoom or the pane size changes.
pub struct StructurePreview {
    service: Arc<PreviewService>,
    texture: Option<TextureHandle>,
    // The structure the texture shows.
    shown: Option<StructureId>,
    camera: OrbitCamera,
    // The structure the camera turn belongs to.
    framed: Option<StructureId>,
    // The last request sent: structure, turn and image size.
    sent: Option<(StructureId, OrbitCamera, [u32; 2])>,
    serial: u64,
    answered: u64,
    render_us: Option<u64>,
    failed: Option<StructureId>,
}

impl StructurePreview {
    /// spike: no Paper sheet yet. The pane's height.
    const HEIGHT: f32 = 320.0;
    /// spike: no Paper sheet yet. The room between the render time and the pane's bottom right corner.
    const TIME_INSET: Vec2 = vec2(8.0, 6.0);
    /// spike: no Paper sheet yet. Degrees the camera turns per point of drag.
    const DEGREES_PER_POINT: f32 = 0.5;
    const TIME: TextStyle = TextStyle::new(TypeSize::Caption, FontFace::Regular);
    const NOTE: TextStyle = TextStyle::new(TypeSize::Label, FontFace::Regular);

    pub fn new(service: Arc<PreviewService>) -> Self {
        Self {
            service,
            texture: None,
            shown: None,
            camera: OrbitCamera::start(),
            framed: None,
            sent: None,
            serial: 0,
            answered: 0,
            render_us: None,
            failed: None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, row: &StructureRow) {
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), Self::HEIGHT), Sense::drag());
        if self.framed != Some(row.id) {
            self.framed = Some(row.id);
            self.camera = OrbitCamera::start();
            self.render_us = None;
        }
        if response.dragged() {
            let delta = response.drag_delta();
            if delta != Vec2::ZERO {
                self.camera = self.camera.turned(-delta.x * Self::DEGREES_PER_POINT, delta.y * Self::DEGREES_PER_POINT);
            }
        }
        if response.contains_pointer() {
            let steps = Self::take_wheel(ui);
            if steps != 0 {
                self.camera = self.camera.zoomed(steps);
            }
        }
        let size = Self::image_size(ui, rect);
        self.request(row, size);
        self.receive(ui, row.id);
        if self.answered < self.serial {
            ui.ctx().request_repaint();
        }
        self.paint(ui, rect, row.id);
    }

    // The picture size in pixels: the pane, capped at the rasteriser's largest picture with the aspect kept.
    fn image_size(ui: &Ui, rect: Rect) -> [u32; 2] {
        let pixels = rect.size() * ui.ctx().pixels_per_point();
        let scale = (PreviewRasterizer::MAX_WIDTH as f32 / pixels.x.max(1.0))
            .min(PreviewRasterizer::MAX_HEIGHT as f32 / pixels.y.max(1.0))
            .min(1.0);
        [(pixels.x * scale).round().max(1.0) as u32, (pixels.y * scale).round().max(1.0) as u32]
    }

    // The wheel steps of this frame, in when positive. The scroll is used up here, so the page behind the pane does
    // not move.
    fn take_wheel(ui: &mut Ui) -> i32 {
        ui.input_mut(|input| {
            let steps = input
                .events
                .iter()
                .map(|event| match event {
                    Event::MouseWheel { delta, .. } if delta.y > 0.0 => 1,
                    Event::MouseWheel { delta, .. } if delta.y < 0.0 => -1,
                    _ => 0,
                })
                .sum();
            input.smooth_scroll_delta = Vec2::ZERO;
            steps
        })
    }

    // Sends a request when the structure, the turn or the image size changed. A full queue is tried again on the
    // next frame.
    fn request(&mut self, row: &StructureRow, size: [u32; 2]) {
        let wanted = (row.id, self.camera, size);
        if self.sent == Some(wanted) {
            return;
        }
        if self.service.submit(self.request_for(row, size, self.serial + 1)).is_ok() {
            self.serial += 1;
            self.sent = Some(wanted);
        }
    }

    fn request_for(&self, row: &StructureRow, size: [u32; 2], serial: u64) -> PreviewRequest {
        PreviewRequest {
            serial,
            structure: row.id,
            file_name: row.file_name.clone(),
            camera: self.camera,
            width: size[0],
            height: size[1],
        }
    }

    fn receive(&mut self, ui: &Ui, structure: StructureId) {
        let Some(frame) = self.service.take() else {
            return;
        };
        self.answered = self.answered.max(frame.serial);
        if frame.structure != structure {
            return;
        }
        match frame.outcome {
            Ok(image) => {
                self.render_us = Some(image.stats.render_us);
                self.upload(ui, &image);
                self.shown = Some(structure);
                self.failed = None;
            }
            Err(_) => self.failed = Some(structure),
        }
    }

    fn upload(&mut self, ui: &Ui, image: &PreviewImage) {
        let pixels = ColorImage::from_rgba_unmultiplied([image.width as usize, image.height as usize], &image.rgba);
        Self::set_texture(ui, &mut self.texture, "stratum-preview", pixels);
    }

    fn set_texture(ui: &Ui, texture: &mut Option<TextureHandle>, name: &str, pixels: ColorImage) {
        match texture {
            Some(texture) => texture.set(pixels, TextureOptions::LINEAR),
            None => *texture = Some(ui.ctx().load_texture(name, pixels, TextureOptions::LINEAR)),
        }
    }

    fn paint(&self, ui: &Ui, rect: Rect, structure: StructureId) {
        let texture = self.texture.as_ref().filter(|_| self.shown == Some(structure));
        let painter = ui.painter();
        Draw::fill(painter, rect, Corner::Control, <color::BgField as ColorToken>::VALUE);
        Draw::stroke(painter, rect, Corner::Control, <color::Line as ColorToken>::VALUE);
        let faint = <color::TextFaint as ColorToken>::VALUE;
        match texture {
            Some(texture) => {
                let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                painter.image(texture.id(), rect, uv, egui::Color32::WHITE);
            }
            None => {
                let note = if self.failed == Some(structure) {
                    StratumMessage::PreviewFailed
                } else {
                    StratumMessage::PreviewDrawing
                };
                let text = Text::message(note);
                Draw::text(painter, rect.center(), Align2::CENTER_CENTER, &text, Self::NOTE, faint, TextFit::Line);
            }
        }
        if let Some(render_us) = self.render_us {
            let ms = UserText::from_model(format!("{:.1}", render_us as f64 / 1000.0));
            let text = Text::format(StratumMessage::PreviewRenderTime, Args::new().user("ms", ms));
            let at = rect.right_bottom() - Self::TIME_INSET;
            Draw::text(painter, at, Align2::RIGHT_BOTTOM, &text, Self::TIME, faint, TextFit::Line);
        }
    }
}
