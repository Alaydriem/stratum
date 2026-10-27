#pragma once

#include <cstdint>
#include <optional>
#include <string>

class Block;
class ContainerClosePacket;
class ContainerOpenPacket;
class FillingContainer;
class IClientInstance;
class Item;
class ItemStack;
class ItemStackResponsePacket;
class Player;
struct ItemStackResponseInfo;

namespace stratum {

// sends the item stack requests that move a stack into the hotbar, the way the game's own inventory screen does:
// a hidden inventory open, one request, a client prediction, then a close once the server opens its side. The
// server's open and its answer to the close are hidden from the game, and the answer to the request is kept for
// the next use call. The prediction is undone when the server refuses the request, when no answer comes in time,
// and when a new request replaces an unanswered one. Holds one request in flight at a time, all fixed size
class StackRequest {
public:
    // the answer to the last request, as StUseProbe::swap_answer reads it
    static constexpr uint8_t ANSWER_NONE     = 0;
    static constexpr uint8_t ANSWER_ACCEPTED = 1;
    static constexpr uint8_t ANSWER_REFUSED  = 2;

    // exchanges main inventory slot `from` (9 to 35) with hotbar slot `to` (0 to 8). A request that cannot be
    // written or sent is answered as refused at once
    static void swap(IClientInstance& client, Player& player, int from, int to);

    // fills hotbar slot `slot` (0 to 8) with a full stack of `block`'s item from the creative menu, destroying
    // what the slot holds. A null block, a block with no creative item, or a request that cannot be sent, is
    // answered as refused at once
    static void pick(IClientInstance& client, Player& player, Block const* block, int slot);

    static uint8_t answer() { return sAnswer; }

    // once per client tick: a request with no answer after REQUEST_TIMEOUT_SECONDS is refused and its prediction
    // undone
    static void expire(double now);

    // the server's answers; the one for the request in flight sets the answer. A refusal undoes the prediction,
    // and an accepted pick takes the net id the server gave the new stack
    static void onResponse(ItemStackResponsePacket const& packet);

    // true hides the server's open of the player inventory this class asked for, and sends the close for it
    static bool onContainerOpen(ContainerOpenPacket const& packet);

    // true hides the server's answer to that close
    static bool onContainerClose(ContainerClosePacket const& packet);

    // drops the request in flight, its prediction undone first, and both windows; on mod disable and world leave
    static void reset();

private:
    // how long the server's open and close answers are waited for
    static constexpr double ANSWER_WINDOW_SECONDS = 2.0;
    // the same wait as core's default EasyPlaceConfig swap_timeout_ms
    static constexpr double REQUEST_TIMEOUT_SECONDS = 0.5;
    // request ids are negative and odd, the way the game's own client ids are; a step of 2 keeps them odd
    static constexpr int FIRST_REQUEST_ID = -100001;
    static constexpr int REQUEST_ID_STEP  = 2;

    static constexpr uint8_t UNDO_NONE = 0;
    static constexpr uint8_t UNDO_SWAP = 1;
    static constexpr uint8_t UNDO_PICK = 2;

    static inline IClientInstance* sClient     = nullptr;
    static inline int              sNextId     = FIRST_REQUEST_ID;
    static inline int              sAwaitId    = 0;
    static inline double           sSentAt     = 0.0;
    static inline uint8_t          sAnswer     = ANSWER_NONE;
    static inline double           sOpenUntil  = 0.0;
    static inline double           sCloseUntil = 0.0;
    static inline int              sOpenedId   = -1;
    // the request bytes, cleared and written again for each request; it keeps its capacity
    static inline std::string      sBytes;

    // the prediction of the request in flight: the hotbar slot it changed, the main inventory slot a swap took
    // from, the item the hotbar slot holds after the prediction, and the stack a pick replaced
    static inline uint8_t     sUndoKind = UNDO_NONE;
    static inline int         sUndoSlot = -1;
    static inline int         sUndoFrom = -1;
    static inline Item const* sUndoItem = nullptr;
    static std::optional<ItemStack> sUndoStack;

    static int  nextId();
    static bool send(IClientInstance& client, Player& player, int requestId);
    static void openInventory(IClientInstance& client, Player& player);
    static void closeInventory(IClientInstance& client, int containerId);
    static void adoptPickNetId(ItemStackResponseInfo const& response);
    // ends the request in flight; with `undo`, puts back what the prediction changed when the slot still holds the
    // predicted item
    static void finish(bool undo);
    // the client's inventory while the predicted slot still holds the predicted item; null otherwise
    static FillingContainer* predictedInventory();
};

} // namespace stratum
