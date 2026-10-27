#include "easyplace/UseDriver.h"

#include "bridge/ModLog.h"
#include "bridge/RustBridge.h"
#include "easyplace/InventoryReader.h"
#include "easyplace/ProbeReader.h"
#include "easyplace/SpotReader.h"
#include "easyplace/StackRequest.h"
#include "easyplace/UseExecutor.h"
#include "state/FrameStatusReader.h"

#include <mc/client/game/IClientInstance.h>
#include <mc/client/player/LocalPlayer.h>
#include <mc/world/gamemode/GameMode.h>
#include <mc/world/level/BlockSource.h>

#include <windows.h>

#include <string>

namespace stratum {

namespace {

constexpr SHORT  KEY_DOWN       = static_cast<SHORT>(0x8000);
constexpr double MS_PER_SECOND  = 1000.0;

} // namespace

bool UseDriver::keyHeld(IClientInstance& client) {
    return (GetAsyncKeyState(VK_RBUTTON) & KEY_DOWN) != 0 && !client.isShowingMenu();
}

void UseDriver::tick(IClientInstance& client) {
    StackRequest::expire(FrameStatusReader::nowSeconds());
    LocalPlayer* player = client.getLocalPlayer();
    if (!player) sPlayer.store(nullptr, std::memory_order_relaxed);
    if (!RustBridge::handle() || !player || !client.getRegion() || !player->mGameMode || !keyHeld(client)) {
        release();
        return;
    }
    double now = FrameStatusReader::nowSeconds();
    if (sHolding && !due(client, now)) return;
    call(client, now, !sHolding);
}

bool UseDriver::press(IClientInstance& client) {
    if (!RustBridge::handle() || !keyHeld(client)) return false;
    double now = FrameStatusReader::nowSeconds();
    if (sHolding && !due(client, now)) return sLastKind != KIND_VANILLA;
    return call(client, now, !sHolding) != KIND_VANILLA;
}

bool UseDriver::blocksGameBuild(Player const& player, bool isSimTick) {
    if (&player != sPlayer.load(std::memory_order_relaxed) || UseExecutor::ownBuild()) return false;
    UseExecutor::noteGameBuild(isSimTick);
    return sHolding && sLastKind != KIND_VANILLA;
}

bool UseDriver::due(IClientInstance& client, double now) {
    LocalPlayer* player   = client.getLocalPlayer();
    GameMode*    gameMode = player ? player->mGameMode.get() : nullptr;
    if (!gameMode) return false;
    double delay = static_cast<double>(gameMode->buildingTickDelay->count()) / MS_PER_SECOND;
    if (now - sLastCall >= delay) return true;
    return sLastAnswer == StackRequest::ANSWER_NONE && StackRequest::answer() != StackRequest::ANSWER_NONE;
}

uint8_t UseDriver::call(IClientInstance& client, double now, bool pressed) {
    StHandle*    handle   = RustBridge::handle();
    LocalPlayer* player   = client.getLocalPlayer();
    BlockSource* region   = client.getRegion();
    GameMode*    gameMode = player ? player->mGameMode.get() : nullptr;
    if (!handle || !player || !region || !gameMode) return KIND_VANILLA;

    StUseProbe probe{};
    probe.time_seconds = now;
    probe.pressed      = pressed;
    probe.held         = true;
    ProbeReader::read(*player, *region, *gameMode, probe);
    probe.swap_answer = StackRequest::answer();

    int32_t  spot[3] = {0, 0, 0};
    bool     hasSpot = false;
    StStatus status  = st_use_aim(handle, &probe, &spot, &hasSpot);
    StSpotWorld world{};
    if (status == ST_STATUS_OK && hasSpot) SpotReader::read(*region, spot, world);

    StUseCommand command{};
    if (status == ST_STATUS_OK) {
        StInventorySlot slots[StInventorySlot_CAPACITY]{};
        uint8_t         selected = 0;
        uint32_t        count    = InventoryReader::read(*player, slots, selected);
        status = st_use_decide(handle, &probe, hasSpot ? &world : nullptr, slots, count, selected, &command);
    }
    if (status != ST_STATUS_OK) command = StUseCommand{};
    if (status != sLastStatus) {
        std::string fields = "{\"status\":" + std::to_string(static_cast<int>(status)) + "}";
        if (status == ST_STATUS_OK) {
            ModLog::info("easy place use call recovered", fields);
        } else {
            ModLog::warn("easy place use call failed", fields);
        }
        sLastStatus = status;
    }

    sPlayer.store(player, std::memory_order_relaxed);
    sHolding  = true;
    sLastCall = now;
    sLastKind = command.kind;
    UseExecutor::run(client, *player, command);
    sLastAnswer = StackRequest::answer();
    return command.kind;
}

void UseDriver::release() {
    sPlayer.store(nullptr, std::memory_order_relaxed);
    if (!sHolding) return;
    sHolding  = false;
    sLastKind = KIND_VANILLA;
    if (StHandle* handle = RustBridge::handle()) st_use_released(handle);
}

void UseDriver::reset() {
    sHolding    = false;
    sLastKind   = KIND_VANILLA;
    sLastCall   = 0.0;
    sLastAnswer = StackRequest::ANSWER_NONE;
    sLastStatus = ST_STATUS_OK;
    sPlayer.store(nullptr, std::memory_order_relaxed);
    StackRequest::reset();
    UseExecutor::reset();
}

} // namespace stratum
