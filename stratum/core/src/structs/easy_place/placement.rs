use super::Face;

// Place against `neighbour` on its `face`, the face that points at `spot`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub spot: [i32; 3],
    pub neighbour: [i32; 3],
    pub face: Face,
}
