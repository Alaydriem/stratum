use stratum_core::structs::easy_place::{Face, GameMode, SwapAnswer, UseHit, UseProbe};

// What the shim reads from the game on one use call, passed to st_use_aim and again to st_use_decide.
// `face` indexes the faces in the order Down, Up, North, South, West, East. `game_mode` is 0 survival,
// 1 creative, 2 anything else. `swap_answer` is 0 for no answer yet, 1 accepted, 2 refused. The hit fields
// are read only when `has_hit` is set. `body_min` and `body_max` are the cells the player body fills, both
// inclusive, read only when `has_body` is set; a box whose min is above its max on any axis reads as no body.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StUseProbe {
    pub time_seconds: f64,
    pub pressed: bool,
    pub held: bool,
    pub eye: [f64; 3],
    pub dir: [f64; 3],
    pub reach: f64,
    pub game_mode: u8,
    pub has_hit: bool,
    pub hit_cell: [i32; 3],
    pub hit_face: u8,
    pub hit_distance: f64,
    pub hit_replaceable: bool,
    pub has_body: bool,
    pub body_min: [i32; 3],
    pub body_max: [i32; 3],
    pub swap_answer: u8,
}

// A value outside the documented range reads as the safe choice: game mode 2, no hit, no answer.
impl From<&StUseProbe> for UseProbe {
    fn from(probe: &StUseProbe) -> Self {
        let game_mode = match probe.game_mode {
            0 => GameMode::Survival,
            1 => GameMode::Creative,
            _ => GameMode::Other,
        };
        let swap_answer = match probe.swap_answer {
            1 => SwapAnswer::Accepted,
            2 => SwapAnswer::Refused,
            _ => SwapAnswer::None,
        };
        let face = Face::ALL.get(usize::from(probe.hit_face)).copied();
        let hit = match (probe.has_hit, face) {
            (true, Some(face)) => Some(UseHit {
                cell: probe.hit_cell,
                face,
                distance: probe.hit_distance,
                replaceable: probe.hit_replaceable,
            }),
            _ => None,
        };
        let ordered = (0..3).all(|axis| probe.body_min[axis] <= probe.body_max[axis]);
        let body = (probe.has_body && ordered).then_some((probe.body_min, probe.body_max));
        Self {
            time_seconds: probe.time_seconds,
            pressed: probe.pressed,
            held: probe.held,
            eye: probe.eye,
            dir: probe.dir,
            reach: probe.reach,
            game_mode,
            hit,
            body,
            swap_answer,
        }
    }
}
