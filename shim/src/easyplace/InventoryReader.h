#pragma once

#include "stratum.h"

#include <cstdint>

class Player;

namespace stratum {

// reads the player's 36 inventory slots, 0 to 8 the hotbar and 9 to 35 the main inventory, as item names and
// counts. Each name views the item's own registered name, so nothing is copied; the views stay valid for the
// st_use_decide call that follows on the game thread. Read-only
class InventoryReader {
public:
    // fills `slots` and `selected`; returns how many slots were filled, 0 when the player has no inventory
    static uint32_t read(Player& player, StInventorySlot (&slots)[StInventorySlot_CAPACITY], uint8_t& selected);
};

} // namespace stratum
