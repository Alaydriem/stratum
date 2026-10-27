use stratum_core::structs::easy_place::InventorySlot;

use crate::services::StBoundary;
use crate::types::StStr;

// One inventory slot, slots 0 to 8 being the hotbar. `item` is the item name, borrowed for the call.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StInventorySlot {
    pub item: StStr,
    pub count: u32,
}

impl StInventorySlot {
    // The most slots one call carries: the hotbar and the main inventory.
    pub const CAPACITY: usize = 36;

    // Fills `out` from `raw` without allocating. Returns how many slots were filled, or None when a name view
    // is null with a length or `raw` is longer than `out`.
    // SAFETY: each `item` is null with len 0, or points to len readable bytes that outlive 'a
    pub unsafe fn fill<'a, const N: usize>(
        raw: &'a [StInventorySlot],
        out: &mut [InventorySlot<'a>; N],
    ) -> Option<usize> {
        if raw.len() > N {
            return None;
        }
        for (target, slot) in out.iter_mut().zip(raw) {
            // SAFETY: see the function contract
            let item = unsafe { StBoundary::slice(slot.item.ptr, slot.item.len) }?;
            *target = InventorySlot { item, count: slot.count };
        }
        Some(raw.len())
    }
}
