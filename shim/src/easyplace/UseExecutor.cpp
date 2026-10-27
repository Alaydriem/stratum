#include "easyplace/UseExecutor.h"

#include "bridge/ModLog.h"
#include "bridge/RustBridge.h"
#include "easyplace/StackRequest.h"
#include "ghost/GhostRenderer.h"
#include "state/FrameStatusReader.h"

#include <mc/client/game/IClientInstance.h>
#include <mc/client/gui/screens/controllers/HudScreenController.h>
#include <mc/client/player/LocalPlayer.h>
#include <mc/common/FacingID.h>
#include <mc/deps/core/math/Vec3.h>
#include <mc/world/ContainerID.h>
#include <mc/world/actor/player/PlayerInventory.h>
#include <mc/world/gamemode/GameMode.h>
#include <mc/world/item/HandSlot.h>
#include <mc/world/level/BlockPos.h>
#include <mc/world/level/Level.h>
#include <mc/world/phys/HitResult.h>
#include <mc/world/phys/HitResultType.h>

#include <string>

namespace stratum {

namespace {

// StUseCommand::face is in Rust's Face::ALL order; the game's face ids are the same order
static_assert(static_cast<int>(FacingID::Down) == 0);
static_assert(static_cast<int>(FacingID::Up) == 1);
static_assert(static_cast<int>(FacingID::North) == 2);
static_assert(static_cast<int>(FacingID::South) == 3);
static_assert(static_cast<int>(FacingID::West) == 4);
static_assert(static_cast<int>(FacingID::East) == 5);

constexpr uint8_t FACE_COUNT   = 6;
constexpr int     HOTBAR_SLOTS = 9;

} // namespace

void UseExecutor::run(IClientInstance& client, LocalPlayer& player, StUseCommand const& command) {
    switch (command.kind) {
    case KIND_SELECT:
        if (!select(player, command.slot) || !place(player, command)) notSent(command);
        break;
    case KIND_SWAP:
        StackRequest::swap(client, player, command.from, command.to);
        break;
    case KIND_PICK:
        StackRequest::pick(client, player, GhostRenderer::paletteBlock(command.palette_id), command.slot);
        break;
    case KIND_PLACE:
        if (!place(player, command)) notSent(command);
        break;
    default:
        break;
    }
}

void UseExecutor::captureHud(HudScreenController& hud) {
    sHud         = &hud;
    sHudSeen     = FrameStatusReader::nowSeconds();
    sNoHudLogged = false;
}

void UseExecutor::forgetHud(ScreenController const& controller) {
    if (sHud && &controller == static_cast<ScreenController const*>(sHud)) sHud = nullptr;
}

void UseExecutor::reset() {
    sHud      = nullptr;
    sHudSeen  = 0.0;
    sOwnBuild = false;
}

// true when the slot is selected after the call
bool UseExecutor::select(LocalPlayer& player, uint8_t slot) {
    PlayerInventory* supplies = player.mInventory.get();
    if (!supplies || slot >= HOTBAR_SLOTS) return false;
    bool fresh = sHud && FrameStatusReader::nowSeconds() - sHudSeen < HUD_FRESH_SECONDS;
    if (!fresh) {
        if (!sNoHudLogged) ModLog::warn("easy place select skipped, no HUD", "{\"slot\":" + std::to_string(slot) + "}");
        sNoHudLogged = true;
        return false;
    }
    sHud->_handleSlotSelection(slot, ContainerID::Inventory);
    if (supplies->mSelected == slot) return true;
    ModLog::warn(
        "easy place select not taken",
        "{\"slot\":" + std::to_string(slot) + ",\"selected\":" + std::to_string(supplies->mSelected) + "}"
    );
    return false;
}

// clicks the neighbour's face that points at the spot, at the point core chose, the way the game's own use does;
// true when the game built
bool UseExecutor::place(LocalPlayer& player, StUseCommand const& command) {
    GameMode* gameMode = player.mGameMode.get();
    if (!gameMode || command.face >= FACE_COUNT) return false;
    BlockPos   neighbour(command.neighbour[0], command.neighbour[1], command.neighbour[2]);
    HitResult& hit    = player.getLevel().getHitResult();
    HitResult  saved  = hit;
    hit.mType         = HitResultType::Tile;
    hit.mBlock        = neighbour;
    hit.mFacing       = command.face;
    hit.mPos          = Vec3(
        float(neighbour.x) + command.click[0],
        float(neighbour.y) + command.click[1],
        float(neighbour.z) + command.click[2]
    );
    sOwnBuild  = true;
    bool built = gameMode->buildBlock(neighbour, command.face, HandSlot::Mainhand, sSimTick);
    sOwnBuild  = false;
    hit        = saved;
    return built;
}

void UseExecutor::notSent(StUseCommand const& command) {
    if (StHandle* handle = RustBridge::handle()) st_use_not_sent(handle, &command.spot);
}

} // namespace stratum
