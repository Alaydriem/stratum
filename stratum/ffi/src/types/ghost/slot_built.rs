use stratum_core::structs::ghost::SlotBuilt;

// The shim's answer for one job: the job's `slot`, `serial` and `generation`, the mesh's GPU bytes, and whether
// the mesh was built.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StSlotBuilt {
    pub slot: u32,
    pub serial: u32,
    pub generation: u64,
    pub bytes: u64,
    pub ok: bool,
}

impl From<&StSlotBuilt> for SlotBuilt {
    fn from(built: &StSlotBuilt) -> Self {
        Self { slot: built.slot, serial: built.serial, generation: built.generation, bytes: built.bytes, ok: built.ok }
    }
}
