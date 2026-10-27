#include "easyplace/StackRequest.h"

#include "bridge/ModLog.h"
#include "state/FrameStatusReader.h"

#include <mc/client/game/IClientInstance.h>
#include <mc/client/player/LocalPlayer.h>
#include <mc/deps/core/utility/BinaryStream.h>
#include <mc/network/MinecraftPacketIds.h>
#include <mc/network/MinecraftPackets.h>
#include <mc/network/Packet.h>
#include <mc/network/PacketSender.h>
#include <mc/network/packet/ContainerClosePacket.h>
#include <mc/network/packet/ContainerOpenPacket.h>
#include <mc/network/packet/InteractPacket.h>
#include <mc/network/packet/ItemStackResponseContainerInfo.h>
#include <mc/network/packet/ItemStackResponseInfo.h>
#include <mc/network/packet/ItemStackResponsePacket.h>
#include <mc/network/packet/ItemStackResponseSlotInfo.h>
#include <mc/world/ContainerID.h>
#include <mc/world/actor/player/Inventory.h>
#include <mc/world/actor/player/Player.h>
#include <mc/world/actor/player/PlayerInventory.h>
#include <mc/world/containers/FullContainerName.h>
#include <mc/world/inventory/FillingContainer.h>
#include <mc/world/inventory/network/ItemStackNetIdVariant.h>
#include <mc/world/item/Item.h>
#include <mc/world/item/ItemInstance.h>
#include <mc/world/item/ItemStack.h>
#include <mc/world/item/registry/CreativeItemEntry.h>
#include <mc/world/item/registry/CreativeItemRegistry.h>
#include <mc/world/item/registry/ItemRegistryRef.h>
#include <mc/world/level/Level.h>
#include <mc/world/level/block/Block.h>
#include <mc/world/level/block/BlockType.h>

#include <memory>
#include <optional>
#include <string>
#include <variant>

namespace stratum {

namespace {

// the container ids of the item stack request protocol
constexpr uchar HOTBAR_CONTAINER         = 28;
constexpr uchar INVENTORY_CONTAINER      = 29;
constexpr uchar CREATED_OUTPUT_CONTAINER = 60;
// the slot of the created output container that a creative craft fills
constexpr uchar CREATED_OUTPUT_SLOT      = 50;

// action types; each action starts with its index in the action variant, then its type, and for most actions the
// two are the same value
constexpr uchar ACTION_TAKE    = 0;
constexpr uchar ACTION_PLACE   = 1;
constexpr uchar ACTION_SWAP    = 2;
constexpr uchar ACTION_DESTROY = 4;
// CraftCreative is type 14 at variant index 12: the deprecated actions 7 and 8 are not in the variant
constexpr uchar ACTION_CRAFT_CREATIVE       = 14;
constexpr uchar ACTION_CRAFT_CREATIVE_INDEX = 12;

constexpr int HOTBAR_SLOTS    = 9;
constexpr int INVENTORY_SLOTS = 36;

double now() { return FrameStatusReader::nowSeconds(); }

FillingContainer* inventoryOf(Player& player) {
    PlayerInventory* supplies = player.mInventory.get();
    return supplies ? supplies->mInventory.get() : nullptr;
}

bool isEmpty(ItemStack const& stack) { return stack.mCount == 0 || !stack.mItem; }

int netIdOf(ItemStack const& stack) {
    auto const* id = std::get_if<ItemStackNetId>(&stack.mNetIdVariant->mVariant.get());
    if (!id) return 0;
    ItemStackNetId copy = *id;
    return static_cast<int>(copy);
}

// one slot of a request: container, no dynamic id, slot, net id (0 for an empty slot)
void writeSlot(BinaryStream& out, uchar container, int slot, int netId) {
    out.writeByte(container, nullptr, nullptr);
    out.writeBool(false, nullptr, nullptr);
    out.writeByte(static_cast<uchar>(slot), nullptr, nullptr);
    out.writeSignedInt(netId, nullptr, nullptr);
}

void writeAction(BinaryStream& out, uchar index, uchar type) {
    out.writeByte(index, nullptr, nullptr);
    out.writeByte(type, nullptr, nullptr);
}

// no strings to filter, and their origin -1
void writeRequestEnd(BinaryStream& out) {
    out.writeUnsignedVarInt(0, nullptr, nullptr);
    out.writeSignedInt(-1, nullptr, nullptr);
}

CreativeItemEntry const* creativeEntry(Player& player, Item const* item, short aux) {
    auto registry = player.getLevel().getItemRegistry().getCreativeItemRegistry();
    if (!registry || !item) return nullptr;
    CreativeItemEntry const* anyAux = nullptr;
    for (CreativeItemEntry const& entry : *registry->mCreativeItems) {
        if (entry.mItemInstance->mItem.get() != item) continue;
        if (entry.mItemInstance->mAuxValue == aux) return &entry;
        if (!anyAux) anyAux = &entry;
    }
    return anyAux;
}

} // namespace

std::optional<ItemStack> StackRequest::sUndoStack;

int StackRequest::nextId() {
    int id = sNextId;
    sNextId -= REQUEST_ID_STEP;
    // starts again long before the id leaves the int range
    if (sNextId > 0 || sNextId < FIRST_REQUEST_ID * 1000) sNextId = FIRST_REQUEST_ID;
    return id;
}

void StackRequest::swap(IClientInstance& client, Player& player, int from, int to) {
    if (sAwaitId != 0) finish(true);
    sAnswer                 = ANSWER_REFUSED;
    FillingContainer* items = inventoryOf(player);
    if (!items || to < 0 || to >= HOTBAR_SLOTS || from < HOTBAR_SLOTS || from >= INVENTORY_SLOTS
        || from >= items->getContainerSize()) {
        ModLog::warn("easy place swap not sent", "{\"from\":" + std::to_string(from) + ",\"to\":" + std::to_string(to) + "}");
        return;
    }
    ItemStack const& source     = items->getItem(from);
    ItemStack const& target     = items->getItem(to);
    bool             empty      = isEmpty(target);
    Item const*      sourceItem = source.mItem.get();
    int              id     = nextId();
    sBytes.clear();
    BinaryStream out(sBytes);
    out.writeUnsignedVarInt(1, nullptr, nullptr);
    out.writeVarInt(id, nullptr, nullptr);
    out.writeUnsignedVarInt(1, nullptr, nullptr);
    // into an empty slot the whole stack is placed; otherwise the two stacks swap
    if (empty) {
        writeAction(out, ACTION_PLACE, ACTION_PLACE);
        out.writeByte(source.mCount, nullptr, nullptr);
    } else {
        writeAction(out, ACTION_SWAP, ACTION_SWAP);
    }
    writeSlot(out, INVENTORY_CONTAINER, from, netIdOf(source));
    writeSlot(out, HOTBAR_CONTAINER, to, empty ? 0 : netIdOf(target));
    writeRequestEnd(out);
    if (!send(client, player, id)) return;
    items->swapSlots(to, from);
    sUndoKind = UNDO_SWAP;
    sUndoSlot = to;
    sUndoFrom = from;
    sUndoItem = sourceItem;
}

void StackRequest::pick(IClientInstance& client, Player& player, Block const* block, int slot) {
    if (sAwaitId != 0) finish(true);
    sAnswer                 = ANSWER_REFUSED;
    FillingContainer* items = inventoryOf(player);
    if (!items || !block || slot < 0 || slot >= HOTBAR_SLOTS) {
        ModLog::warn("easy place pick not sent", "{\"slot\":" + std::to_string(slot) + "}");
        return;
    }
    ItemInstance             wanted = block->getBlockType().asItemInstance(*block, nullptr);
    Item const*              item   = wanted.mItem.get();
    CreativeItemEntry const* entry  = creativeEntry(player, item, wanted.mAuxValue);
    if (!entry) {
        ModLog::warn("easy place pick has no creative item", "{\"block\":" + ModLog::quote(block->getTypeName()) + "}");
        return;
    }
    ItemStack const& held   = items->getItem(slot);
    bool             empty  = isEmpty(held);
    int              amount = entry->mItemInstance->getMaxStackSize();
    int              id     = nextId();
    sBytes.clear();
    BinaryStream out(sBytes);
    out.writeUnsignedVarInt(1, nullptr, nullptr);
    out.writeVarInt(id, nullptr, nullptr);
    out.writeUnsignedVarInt(empty ? 2 : 3, nullptr, nullptr);
    if (!empty) {
        writeAction(out, ACTION_DESTROY, ACTION_DESTROY);
        out.writeByte(held.mCount, nullptr, nullptr);
        writeSlot(out, HOTBAR_CONTAINER, slot, netIdOf(held));
    }
    writeAction(out, ACTION_CRAFT_CREATIVE_INDEX, ACTION_CRAFT_CREATIVE);
    out.writeUnsignedVarInt(entry->mCreativeNetId->mRawId, nullptr, nullptr);
    out.writeByte(1, nullptr, nullptr);
    writeAction(out, ACTION_TAKE, ACTION_TAKE);
    out.writeByte(static_cast<uchar>(amount), nullptr, nullptr);
    // the created stack's net id is the request's own id
    writeSlot(out, CREATED_OUTPUT_CONTAINER, CREATED_OUTPUT_SLOT, id);
    writeSlot(out, HOTBAR_CONTAINER, slot, 0);
    writeRequestEnd(out);
    if (!send(client, player, id)) return;
    sUndoStack.emplace(held);
    ItemStack predicted;
    predicted.reinit(*item, amount, entry->mItemInstance->mAuxValue);
    items->setItem(slot, predicted);
    sUndoKind = UNDO_PICK;
    sUndoSlot = slot;
    sUndoItem = item;
}

// reads the bytes into a request packet the way the network layer would, opens the inventory and sends the request
bool StackRequest::send(IClientInstance& client, Player& player, int requestId) {
    std::shared_ptr<Packet> packet = MinecraftPackets::createPacket(MinecraftPacketIds::ItemStackRequest);
    if (!packet) {
        ModLog::warn("easy place stack request not created");
        return false;
    }
    ReadOnlyBinaryStream in(sBytes, true);
    auto                 read = packet->read(in);
    if (!read) {
        ModLog::warn(
            "easy place stack request not read",
            "{\"code\":" + std::to_string(read.error().code().value()) + ",\"read_to\":" + std::to_string(in.mReadPointer)
                + "}"
        );
        return false;
    }
    sClient = &client;
    openInventory(client, player);
    sAwaitId = requestId;
    sSentAt  = now();
    sAnswer  = ANSWER_NONE;
    client.getPacketSender().send(*packet);
    return true;
}

void StackRequest::openInventory(IClientInstance& client, Player& player) {
    std::shared_ptr<Packet> packet = MinecraftPackets::createPacket(MinecraftPacketIds::Interact);
    if (!packet) return;
    auto* interact      = static_cast<InteractPacket*>(packet.get());
    interact->mAction   = InteractPacketPayload::Action::OpenInventory;
    interact->mTargetId = player.getRuntimeID();
    client.getPacketSender().send(*packet);
    sOpenUntil = now() + ANSWER_WINDOW_SECONDS;
}

void StackRequest::closeInventory(IClientInstance& client, int containerId) {
    std::shared_ptr<Packet> packet = MinecraftPackets::createPacket(MinecraftPacketIds::ContainerClose);
    if (!packet) return;
    auto* close                  = static_cast<ContainerClosePacket*>(packet.get());
    close->mContainerId          = static_cast<ContainerID>(containerId);
    close->mContainerType        = SharedTypes::Legacy::ContainerType::None;
    close->mServerInitiatedClose = false;
    client.getPacketSender().send(*packet);
    sCloseUntil = now() + ANSWER_WINDOW_SECONDS;
}

void StackRequest::onResponse(ItemStackResponsePacket const& packet) {
    if (sAwaitId == 0) return;
    for (ItemStackResponseInfo const& response : *packet.mResponses) {
        if (response.mClientRequestId->mRawId != sAwaitId) continue;
        bool accepted = static_cast<int>(response.mResult) == 0;
        sAnswer       = accepted ? ANSWER_ACCEPTED : ANSWER_REFUSED;
        if (accepted) {
            if (sUndoKind == UNDO_PICK) adoptPickNetId(response);
            finish(false);
            return;
        }
        ModLog::info(
            "easy place stack request refused",
            "{\"result\":" + std::to_string(static_cast<int>(response.mResult)) + "}"
        );
        finish(true);
        return;
    }
}

void StackRequest::expire(double now) {
    if (sAwaitId == 0 || now - sSentAt < REQUEST_TIMEOUT_SECONDS) return;
    ModLog::info("easy place stack request not answered");
    finish(true);
    sAnswer = ANSWER_REFUSED;
}

void StackRequest::reset() {
    finish(sAwaitId != 0);
    sAnswer     = ANSWER_NONE;
    sOpenUntil  = 0.0;
    sCloseUntil = 0.0;
    sOpenedId   = -1;
    sClient     = nullptr;
}

void StackRequest::finish(bool undo) {
    FillingContainer* items = undo ? predictedInventory() : nullptr;
    if (items && sUndoKind == UNDO_SWAP) items->swapSlots(sUndoSlot, sUndoFrom);
    if (items && sUndoKind == UNDO_PICK && sUndoStack) items->setItem(sUndoSlot, *sUndoStack);
    sAwaitId  = 0;
    sUndoKind = UNDO_NONE;
    sUndoSlot = -1;
    sUndoFrom = -1;
    sUndoItem = nullptr;
    sUndoStack.reset();
}

FillingContainer* StackRequest::predictedInventory() {
    if (sUndoKind == UNDO_NONE || sUndoSlot < 0) return nullptr;
    LocalPlayer*      player = sClient ? sClient->getLocalPlayer() : nullptr;
    FillingContainer* items  = player ? inventoryOf(*player) : nullptr;
    if (!items || sUndoSlot >= items->getContainerSize()) return nullptr;
    return items->getItem(sUndoSlot).mItem.get() == sUndoItem ? items : nullptr;
}

// later requests name the stack by the net id the server gave it, so the predicted stack takes that id; a slot
// that no longer holds the picked item is left alone
void StackRequest::adoptPickNetId(ItemStackResponseInfo const& response) {
    FillingContainer* items = predictedInventory();
    if (!items) return;
    for (ItemStackResponseContainerInfo const& container : *response.mContainers) {
        if (static_cast<int>(container.mFullContainerName->mName) != HOTBAR_CONTAINER) continue;
        for (ItemStackResponseSlotInfo const& slot : *container.mSlots) {
            if (static_cast<int>(slot.mSlot) != sUndoSlot) continue;
            auto& stack                   = const_cast<ItemStack&>(items->getItem(sUndoSlot));
            stack.mNetIdVariant->mVariant = *slot.mItemStackNetId;
        }
    }
}

bool StackRequest::onContainerOpen(ContainerOpenPacket const& packet) {
    if (now() > sOpenUntil || packet.mType != SharedTypes::Legacy::ContainerType::Inventory) return false;
    sOpenUntil = 0.0;
    sOpenedId  = static_cast<int>(packet.mContainerId);
    if (sClient) closeInventory(*sClient, sOpenedId);
    return true;
}

bool StackRequest::onContainerClose(ContainerClosePacket const& packet) {
    if (now() > sCloseUntil || static_cast<int>(packet.mContainerId) != sOpenedId) return false;
    sCloseUntil = 0.0;
    return true;
}

} // namespace stratum
