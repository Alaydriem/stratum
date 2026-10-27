use std::time::{Duration, Instant};

use super::{PreviewMesher, PreviewRasterizer, TexturePack};
use crate::errors::{PreviewError, StructureError};
use crate::services::StructureParser;
use crate::structs::library::StructureId;
use crate::structs::preview::{PreviewFrame, PreviewImage, PreviewMesh, PreviewRequest, PreviewSource, PreviewStats};

// The state of the preview thread: the texture pack, read on the first request, the mesh of the last structure
// and the rasteriser's buffers.
// Bound: one mesh is kept; a request for another structure replaces it.
pub(crate) struct PreviewWorker {
    source: PreviewSource,
    pack: Option<TexturePack>,
    mesh: Option<(StructureId, PreviewMesh)>,
    rasterizer: PreviewRasterizer,
    last_log: Option<Instant>,
}

impl PreviewWorker {
    const LOG_EVERY: Duration = Duration::from_millis(250);

    pub(crate) fn new(source: PreviewSource) -> Self {
        Self { source, pack: None, mesh: None, rasterizer: PreviewRasterizer::new(), last_log: None }
    }

    pub(crate) fn handle(&mut self, request: &PreviewRequest) -> PreviewFrame {
        let outcome = self.render(request);
        if let Err(error) = &outcome {
            curia::warn!("preview render failed", { structure: request.structure.0, error: error.to_string() });
        }
        PreviewFrame { serial: request.serial, structure: request.structure, outcome }
    }

    fn render(&mut self, request: &PreviewRequest) -> Result<PreviewImage, PreviewError> {
        let pack = self.pack.get_or_insert_with(|| Self::load_pack(&self.source));
        let mut mesh_us = 0;
        let mesh = match self.mesh.take() {
            Some((id, mesh)) if id == request.structure => mesh,
            _ => {
                let started = Instant::now();
                let path = self.source.structures_dir.join(&request.file_name);
                let bytes = std::fs::read(&path).map_err(|e| StructureError::Unreadable(e.to_string()))?;
                let structure = StructureParser::parse(&bytes)?;
                let mesh = PreviewMesher::build(&structure, pack)?;
                mesh_us = Self::micros(started);
                curia::info!("preview mesh built", {
                    structure: request.structure.0,
                    mesh_us: mesh_us,
                    quads: mesh.quads.len(),
                    textures: mesh.textures.len(),
                    missing_blocks: mesh.missing,
                    truncated: mesh.truncated,
                });
                mesh
            }
        };
        let started = Instant::now();
        let triangles = self.rasterizer.render(&mesh, request.camera, request.width, request.height);
        let render_us = Self::micros(started);
        let stats = PreviewStats {
            render_us,
            mesh_us,
            quads: mesh.quads.len(),
            triangles,
            width: self.rasterizer.width(),
            height: self.rasterizer.height(),
            size: mesh.size,
        };
        let image =
            PreviewImage { width: stats.width, height: stats.height, rgba: self.rasterizer.pixels().to_vec(), stats };
        self.mesh = Some((request.structure, mesh));
        if self.last_log.is_none_or(|at| at.elapsed() >= Self::LOG_EVERY) {
            self.last_log = Some(Instant::now());
            curia::info!("preview render", {
                structure: request.structure.0,
                render_us: stats.render_us,
                triangles: stats.triangles,
                quads: stats.quads,
                width: stats.width,
                height: stats.height,
                size_x: stats.size.x,
                size_y: stats.size.y,
                size_z: stats.size.z,
                yaw: request.camera.yaw,
                pitch: request.camera.pitch,
            });
        }
        Ok(image)
    }

    fn load_pack(source: &PreviewSource) -> TexturePack {
        let started = Instant::now();
        let pack = match &source.packs_dir {
            Some(dir) => TexturePack::load(dir),
            None => TexturePack::empty(),
        };
        curia::info!("preview textures loaded", {
            load_us: Self::micros(started),
            packs: pack.pack_count(),
            blocks: pack.block_count(),
            texture_files: pack.file_count(),
            archive_bytes: pack.archive_bytes(),
            found: source.packs_dir.is_some(),
        });
        pack
    }

    fn micros(started: Instant) -> u64 {
        u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}
