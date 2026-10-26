use crate::types::StBytes;

// One palette entry: the little-endian NBT compound `{ name, states, version }` the game's block registry reads.
// Empty when core could not write the entry; the shim then draws the game's unknown block.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StGhostBlock {
    pub nbt: StBytes,
}
