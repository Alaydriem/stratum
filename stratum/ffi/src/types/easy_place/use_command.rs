use stratum_core::structs::easy_place::{Placement, UseCommand};

// What the shim does with the use call. `kind` is 0 vanilla (let the game handle the use), 1 block (cancel the
// use, do nothing else), 2 select (change to `slot`, then place), 3 swap (exchange the stacks in `from` and
// `to`, place nothing), 4 pick (creative pick of `palette_id` into `slot`, place nothing), 5 place. A place or
// select places at `spot` against `neighbour` on `face`, the face of the neighbour that points at the spot,
// with the faces in the order Down, Up, North, South, West, East. `click` is the click point inside the
// neighbour block, from its minimum corner. Fields a kind does not use are zero.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StUseCommand {
    pub kind: u8,
    pub slot: u8,
    pub from: u8,
    pub to: u8,
    pub palette_id: u16,
    pub spot: [i32; 3],
    pub neighbour: [i32; 3],
    pub face: u8,
    // The click point inside the neighbour block, from its minimum corner, each axis 0.0 to 1.0.
    pub click: [f32; 3],
}

impl StUseCommand {
    const VANILLA: u8 = 0;
    const BLOCK: u8 = 1;
    const SELECT: u8 = 2;
    const SWAP: u8 = 3;
    const PICK: u8 = 4;
    const PLACE: u8 = 5;

    // The command that leaves the use to the game.
    pub fn vanilla() -> Self {
        Self::of(Self::VANILLA)
    }

    fn of(kind: u8) -> Self {
        Self { kind, slot: 0, from: 0, to: 0, palette_id: 0, spot: [0; 3], neighbour: [0; 3], face: 0, click: [0.0; 3] }
    }

    fn placed(kind: u8, slot: u8, placement: Placement) -> Self {
        Self {
            slot,
            spot: placement.spot,
            neighbour: placement.neighbour,
            face: placement.face.index() as u8,
            click: placement.click,
            ..Self::of(kind)
        }
    }
}

impl From<UseCommand> for StUseCommand {
    fn from(command: UseCommand) -> Self {
        match command {
            UseCommand::Vanilla => Self::of(Self::VANILLA),
            UseCommand::Block => Self::of(Self::BLOCK),
            UseCommand::Select { slot, place } => Self::placed(Self::SELECT, slot, place),
            UseCommand::Swap { from, to } => Self { from, to, ..Self::of(Self::SWAP) },
            UseCommand::Pick { palette_id, slot } => Self { palette_id, slot, ..Self::of(Self::PICK) },
            UseCommand::Place(place) => Self::placed(Self::PLACE, 0, place),
        }
    }
}
