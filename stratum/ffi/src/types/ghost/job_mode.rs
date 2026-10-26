use stratum_core::structs::ghost::GhostJobMode;

// How a job lands in its slot. Replace builds the whole slot mesh. Append adds the job's mesh to the slot mesh of
// its group. Finish does the same and completes the slot mesh.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StGhostJobMode {
    Replace = 0,
    Append = 1,
    Finish = 2,
}

impl From<GhostJobMode> for StGhostJobMode {
    fn from(mode: GhostJobMode) -> Self {
        match mode {
            GhostJobMode::Replace => Self::Replace,
            GhostJobMode::Append => Self::Append,
            GhostJobMode::Finish => Self::Finish,
        }
    }
}
