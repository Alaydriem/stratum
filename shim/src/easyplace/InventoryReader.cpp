#include "easyplace/InventoryReader.h"

#include "bridge/RustBridge.h"

#include <mc/world/actor/player/Inventory.h>
#include <mc/world/actor/player/Player.h>
#include <mc/world/actor/player/PlayerInventory.h>
#include <mc/world/inventory/FillingContainer.h>
#include <mc/world/item/Item.h>
#include <mc/world/item/ItemStack.h>

#include <algorithm>

namespace stratum {

uint32_t InventoryReader::read(Player& player, StInventorySlot (&slots)[StInventorySlot_CAPACITY], uint8_t& selected) {
    selected                    = 0;
    PlayerInventory* supplies   = player.mInventory.get();
    if (!supplies || !supplies->mInventory) return 0;
    FillingContainer const& container = *supplies->mInventory;
    int count = std::min(container.getContainerSize(), static_cast<int>(StInventorySlot_CAPACITY));
    for (int i = 0; i < count; ++i) {
        ItemStack const& stack = container.getItem(i);
        Item const*      item  = stack.mItem.get();
        if (!item || stack.mCount == 0) {
            slots[i] = StInventorySlot{};
            continue;
        }
        slots[i] = StInventorySlot{RustBridge::view(item->mFullName->getString()), stack.mCount};
    }
    if (supplies->mSelected >= 0 && supplies->mSelected < count) selected = static_cast<uint8_t>(supplies->mSelected);
    return count > 0 ? static_cast<uint32_t>(count) : 0;
}

} // namespace stratum
