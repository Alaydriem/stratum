use std::collections::HashMap;
use std::sync::Arc;

use super::{ShapeRules, TexturePack};
use crate::errors::PreviewError;
use crate::structs::preview::{BlockFace, BlockLook, BlockShape, BlockTexture, Occlusion, PreviewMesh, PreviewQuad};
use crate::structs::structure::{BlockLayer, StateValue, Structure};

// Builds the quads of a structure's visible faces. A face against a neighbour that hides it is left out.
pub struct PreviewMesher;

impl PreviewMesher {
    // The longest side the mesh can place: corners are i16 sixteenths.
    pub const MAX_SIDE: u32 = 2047;
    const MISSING: [u8; 4] = [160, 160, 160, 255];
    // The light of a cross plant, which the game draws a little darker than a top face.
    const CROSS_SHADE: u8 = 230;

    pub fn build(structure: &Structure, pack: &mut TexturePack) -> Result<PreviewMesh, PreviewError> {
        let size = structure.size;
        if size.x > Self::MAX_SIDE || size.y > Self::MAX_SIDE || size.z > Self::MAX_SIDE {
            return Err(PreviewError::TooLarge);
        }
        let mut mesh = PreviewMesh::new(size, vec![Arc::new(BlockTexture::solid(Self::MISSING))]);
        let looks = Self::looks(structure, pack, &mut mesh);
        let occlusion: Vec<Occlusion> = looks.iter().map(|look| look.occlusion).collect();
        let cells = structure.primary.as_slice();
        let (sx, sy, sz) = (size.x as usize, size.y as usize, size.z as usize);
        let neighbour = |x: usize, y: usize, z: usize, face: BlockFace| -> Occlusion {
            let [dx, dy, dz] = face.offset();
            let (nx, ny, nz) = (x as i64 + i64::from(dx), y as i64 + i64::from(dy), z as i64 + i64::from(dz));
            if nx < 0 || ny < 0 || nz < 0 || nx >= sx as i64 || ny >= sy as i64 || nz >= sz as i64 {
                return Occlusion::None;
            }
            let index = (nx as usize * sy + ny as usize) * sz + nz as usize;
            match cells.get(index) {
                Some(&palette) if palette != BlockLayer::NONE => {
                    occlusion.get(usize::from(palette)).copied().unwrap_or(Occlusion::None)
                }
                _ => Occlusion::None,
            }
        };
        for x in 0..sx {
            for y in 0..sy {
                for z in 0..sz {
                    let Some(&palette) = cells.get((x * sy + y) * sz + z) else {
                        continue;
                    };
                    let Some(look) = looks.get(usize::from(palette)) else {
                        continue;
                    };
                    if mesh.quads.len() + 12 > PreviewMesh::MAX_QUADS {
                        mesh.truncated = true;
                        return Ok(mesh);
                    }
                    let cell = [x as i16 * 16, y as i16 * 16, z as i16 * 16];
                    let hidden = |face: BlockFace| look.occlusion.hidden_by(neighbour(x, y, z, face));
                    match look.shape {
                        BlockShape::Hidden => {}
                        BlockShape::Cube => {
                            Self::emit_box(&mut mesh, cell, [0, 0, 0], [16, 16, 16], look, &hidden, None)
                        }
                        BlockShape::Box { min, max } => Self::emit_box(&mut mesh, cell, min, max, look, &hidden, None),
                        BlockShape::Stairs { upside_down, high_side } => {
                            Self::emit_stairs(&mut mesh, cell, upside_down, high_side, look, &hidden)
                        }
                        BlockShape::Cross => Self::emit_cross(&mut mesh, cell, look),
                    }
                }
            }
        }
        Ok(mesh)
    }

    // One look per palette entry, with its textures added to the mesh's table.
    fn looks(structure: &Structure, pack: &mut TexturePack, mesh: &mut PreviewMesh) -> Vec<BlockLook> {
        let mut table: HashMap<String, u16> = HashMap::new();
        let mut looks = Vec::with_capacity(structure.palette.len());
        for block in &structure.palette {
            let (shape, occlusion) = ShapeRules::shape(block);
            if shape == BlockShape::Hidden {
                looks.push(BlockLook::hidden());
                continue;
            }
            let Some(mut keys) = pack.face_keys(&block.name) else {
                mesh.missing += 1;
                looks.push(BlockLook::new(shape, [0; 6], occlusion));
                continue;
            };
            // a log lying along X or Z shows its rings on the ends instead of the top and bottom
            match block.states.get("pillar_axis") {
                Some(StateValue::Text(axis)) if axis == "x" => {
                    keys.swap(BlockFace::Up.index(), BlockFace::East.index());
                    keys.swap(BlockFace::Down.index(), BlockFace::West.index());
                }
                Some(StateValue::Text(axis)) if axis == "z" => {
                    keys.swap(BlockFace::Up.index(), BlockFace::North.index());
                    keys.swap(BlockFace::Down.index(), BlockFace::South.index());
                }
                _ => {}
            }
            // the upper half of a two block plant uses the top texture, the lower half the bottom one
            if shape == BlockShape::Cross {
                let upper =
                    matches!(block.states.get("upper_block_bit"), Some(StateValue::Bool(true) | StateValue::Int(1)));
                let has_halves = block.states.get("upper_block_bit").is_some();
                let face = if has_halves && !upper { BlockFace::Down } else { BlockFace::Up };
                let key = keys[face.index()].clone();
                keys = std::array::from_fn(|_| key.clone());
            }
            let mut textures = [0u16; 6];
            let mut found_any = false;
            for (slot, key) in textures.iter_mut().zip(keys.iter()) {
                if let Some(&index) = table.get(key) {
                    *slot = index;
                    found_any |= index != 0;
                    continue;
                }
                let index = match pack.texture(key) {
                    Some(texture) if mesh.textures.len() < usize::from(u16::MAX) => {
                        mesh.textures.push(texture);
                        (mesh.textures.len() - 1) as u16
                    }
                    _ => 0,
                };
                table.insert(key.clone(), index);
                found_any |= index != 0;
                *slot = index;
            }
            if !found_any {
                mesh.missing += 1;
            }
            looks.push(BlockLook::new(shape, textures, occlusion));
        }
        looks
    }

    fn emit_stairs(
        mesh: &mut PreviewMesh,
        cell: [i16; 3],
        upside_down: bool,
        high_side: BlockFace,
        look: &BlockLook,
        hidden: &impl Fn(BlockFace) -> bool,
    ) {
        let (slab_min, slab_max, step_y, inner) =
            if upside_down { (8, 16, (0, 8), BlockFace::Up) } else { (0, 8, (8, 16), BlockFace::Down) };
        Self::emit_box(mesh, cell, [0, slab_min, 0], [16, slab_max, 16], look, hidden, None);
        let (min, max) = match high_side {
            BlockFace::West => ([0, step_y.0, 0], [8, step_y.1, 16]),
            BlockFace::South => ([0, step_y.0, 8], [16, step_y.1, 16]),
            BlockFace::North => ([0, step_y.0, 0], [16, step_y.1, 8]),
            _ => ([8, step_y.0, 0], [16, step_y.1, 16]),
        };
        Self::emit_box(mesh, cell, min, max, look, hidden, Some(inner));
    }

    // The faces of one box; a face on the cell's side that the neighbour hides, and the `skip` face, are left out.
    fn emit_box(
        mesh: &mut PreviewMesh,
        cell: [i16; 3],
        min: [u8; 3],
        max: [u8; 3],
        look: &BlockLook,
        hidden: &impl Fn(BlockFace) -> bool,
        skip: Option<BlockFace>,
    ) {
        let [x0, y0, z0] = min.map(i16::from);
        let [x1, y1, z1] = max.map(i16::from);
        for face in BlockFace::ALL {
            if skip == Some(face) {
                continue;
            }
            let on_side = match face {
                BlockFace::Up => y1 == 16,
                BlockFace::Down => y0 == 0,
                BlockFace::North => z0 == 0,
                BlockFace::South => z1 == 16,
                BlockFace::West => x0 == 0,
                BlockFace::East => x1 == 16,
            };
            if on_side && hidden(face) {
                continue;
            }
            // local corners, then the texture position of each from where it sits in the cell
            let local: [[i16; 3]; 4] = match face {
                BlockFace::Up => [[x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]],
                BlockFace::Down => [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                BlockFace::North => [[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]],
                BlockFace::South => [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
                BlockFace::West => [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
                BlockFace::East => [[x1, y0, z0], [x1, y0, z1], [x1, y1, z1], [x1, y1, z0]],
            };
            let uvs = local.map(|[x, y, z]| {
                let (u, v) = match face {
                    BlockFace::Up => (x, z),
                    BlockFace::Down => (x, 16 - z),
                    BlockFace::North => (16 - x, 16 - y),
                    BlockFace::South => (x, 16 - y),
                    BlockFace::West => (z, 16 - y),
                    BlockFace::East => (16 - z, 16 - y),
                };
                [u as u8, v as u8]
            });
            let [nx, ny, nz] = face.offset();
            mesh.quads.push(PreviewQuad {
                corners: local.map(|[x, y, z]| [cell[0] + x, cell[1] + y, cell[2] + z]),
                uvs,
                texture: look.textures[face.index()],
                shade: face.shade(),
                normal: [nx as i8, ny as i8, nz as i8],
            });
        }
    }

    fn emit_cross(mesh: &mut PreviewMesh, cell: [i16; 3], look: &BlockLook) {
        let uvs = [[0, 16], [16, 16], [16, 0], [0, 0]];
        let planes: [[[i16; 3]; 4]; 2] =
            [[[0, 0, 0], [16, 0, 16], [16, 16, 16], [0, 16, 0]], [[16, 0, 0], [0, 0, 16], [0, 16, 16], [16, 16, 0]]];
        for plane in planes {
            mesh.quads.push(PreviewQuad {
                corners: plane.map(|[x, y, z]| [cell[0] + x, cell[1] + y, cell[2] + z]),
                uvs,
                texture: look.textures[BlockFace::Up.index()],
                shade: Self::CROSS_SHADE,
                normal: [0, 0, 0],
            });
        }
    }
}
