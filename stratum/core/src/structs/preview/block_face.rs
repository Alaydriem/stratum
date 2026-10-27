// One side of a block cell. North is -Z, west is -X, as in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFace {
    Up,
    Down,
    North,
    South,
    West,
    East,
}

impl BlockFace {
    pub const ALL: [BlockFace; 6] =
        [BlockFace::Up, BlockFace::Down, BlockFace::North, BlockFace::South, BlockFace::West, BlockFace::East];

    pub fn index(self) -> usize {
        self as usize
    }

    // The step to the neighbour cell on this side.
    pub fn offset(self) -> [i32; 3] {
        match self {
            Self::Up => [0, 1, 0],
            Self::Down => [0, -1, 0],
            Self::North => [0, 0, -1],
            Self::South => [0, 0, 1],
            Self::West => [-1, 0, 0],
            Self::East => [1, 0, 0],
        }
    }

    // The flat light the game gives this side, out of 255: the top full, the bottom half, the sides between.
    pub fn shade(self) -> u8 {
        match self {
            Self::Up => 255,
            Self::Down => 128,
            Self::North | Self::South => 204,
            Self::West | Self::East => 153,
        }
    }
}
