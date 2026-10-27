use std::sync::Arc;

use super::{BlockTexture, PreviewQuad};
use crate::structs::structure::StructureSize;

// The faces of one structure that can be seen, with the textures they use.
// Bound: at most `MAX_QUADS` quads; a structure with more is cut and `truncated` is set.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewMesh {
    pub size: StructureSize,
    pub quads: Vec<PreviewQuad>,
    // Index 0 is the grey stand-in for a missing texture.
    pub textures: Vec<Arc<BlockTexture>>,
    // Palette entries with no texture found.
    pub missing: u32,
    pub truncated: bool,
}

impl PreviewMesh {
    pub const MAX_QUADS: usize = 1_000_000;

    pub fn new(size: StructureSize, textures: Vec<Arc<BlockTexture>>) -> Self {
        Self { size, quads: Vec::new(), textures, missing: 0, truncated: false }
    }

    pub fn triangles(&self) -> usize {
        self.quads.len() * 2
    }
}
