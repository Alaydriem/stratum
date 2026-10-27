use stratum_core::structs::easy_place::SpotWorld;

// What surrounds the spot st_use_aim returned: `against[i]` is true when the neighbour on face i can be
// placed against, with the faces in the order Down, Up, North, South, West, East.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StSpotWorld {
    pub against: [bool; 6],
}

impl From<&StSpotWorld> for SpotWorld {
    fn from(world: &StSpotWorld) -> Self {
        Self { against: world.against }
    }
}
