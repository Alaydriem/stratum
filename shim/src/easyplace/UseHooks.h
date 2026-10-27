#pragma once

#include "easyplace/StackRequest.h"
#include "easyplace/UseDriver.h"
#include "easyplace/UseExecutor.h"

#include <ll/api/memory/Hook.h>
#include <mc/client/game/ClientInputCallbacks.h>
#include <mc/client/game/IClientInstance.h>
#include <mc/client/gui/DirtyFlag.h>
#include <mc/client/gui/screens/ScreenController.h>
#include <mc/client/gui/screens/controllers/HudScreenController.h>
#include <mc/client/input/BuildActionIntention.h>
#include <mc/client/network/ClientNetworkHandler.h>
#include <mc/client/network/LegacyClientNetworkHandler.h>
#include <mc/network/packet/ContainerClosePacket.h>
#include <mc/network/packet/ContainerOpenPacket.h>
#include <mc/network/packet/ItemStackResponseContainerInfo.h>
#include <mc/network/packet/ItemStackResponseInfo.h>
#include <mc/network/packet/ItemStackResponsePacket.h>
#include <mc/network/packet/ItemStackResponseSlotInfo.h>
#include <mc/world/gamemode/GameMode.h>
#include <mc/world/item/HandSlot.h>
#include <mc/world/level/BlockPos.h>
#include <mc/world/phys/HitResult.h>

namespace stratum {

// the use press; held repeats do not come through here. A press whose use call is not left to the game is
// cancelled
LL_STATIC_HOOK(
    UseBuildActionHook,
    ll::memory::HookPriority::Normal,
    &ClientInputCallbacks::handleBuildAction,
    bool,
    IClientInstance&      client,
    BuildActionIntention& intention,
    HitResult const&      solidHitResult,
    HitResult const&      liquidHitResult
) {
    if (UseDriver::press(client)) return true;
    return origin(client, intention, solidHitResult, liquidHitResult);
}

// every block build of a game mode; the local player's held repeats come here. Its own builds are skipped while
// the hold's last use call was not left to the game
LL_TYPE_INSTANCE_HOOK(
    UseBuildBlockHook,
    ll::memory::HookPriority::Normal,
    GameMode,
    &GameMode::$buildBlock,
    bool,
    BlockPos const& pos,
    uchar           face,
    HandSlot        handSlot,
    bool const      isSimTick
) {
    if (UseDriver::blocksGameBuild(this->mPlayer, isSimTick)) return false;
    return origin(pos, face, handSlot, isSimTick);
}

// the HUD screen's tick, for the controller the hotbar select goes through
LL_TYPE_INSTANCE_HOOK(
    UseHudTickHook,
    ll::memory::HookPriority::Normal,
    HudScreenController,
    &HudScreenController::$tick,
    ui::DirtyFlag
) {
    UseExecutor::captureHud(*this);
    return origin();
}

// every screen controller's destructor; the captured HUD controller is forgotten before it is freed
LL_TYPE_INSTANCE_HOOK(
    UseScreenGoneHook,
    ll::memory::HookPriority::Normal,
    ScreenController,
    &ScreenController::$dtor,
    void
) {
    UseExecutor::forgetHud(*this);
    origin();
}

// the server's answers to item stack requests
LL_TYPE_INSTANCE_HOOK(
    UseStackResponseHook,
    ll::memory::HookPriority::Normal,
    ClientNetworkHandler,
    static_cast<void (ClientNetworkHandler::*)(NetworkIdentifier const&, ItemStackResponsePacket const&)>(
        &ClientNetworkHandler::$handle
    ),
    void,
    NetworkIdentifier const&       source,
    ItemStackResponsePacket const& packet
) {
    StackRequest::onResponse(packet);
    origin(source, packet);
}

// the server's container opens; the inventory open StackRequest asked for is hidden from the game
LL_TYPE_INSTANCE_HOOK(
    UseContainerOpenHook,
    ll::memory::HookPriority::Normal,
    LegacyClientNetworkHandler,
    static_cast<void (LegacyClientNetworkHandler::*)(NetworkIdentifier const&, ContainerOpenPacket const&)>(
        &LegacyClientNetworkHandler::$handle
    ),
    void,
    NetworkIdentifier const&   source,
    ContainerOpenPacket const& packet
) {
    if (StackRequest::onContainerOpen(packet)) return;
    origin(source, packet);
}

// the server's container closes; its answer to StackRequest's close is hidden from the game
LL_TYPE_INSTANCE_HOOK(
    UseContainerCloseHook,
    ll::memory::HookPriority::Normal,
    LegacyClientNetworkHandler,
    static_cast<void (LegacyClientNetworkHandler::*)(NetworkIdentifier const&, ContainerClosePacket const&)>(
        &LegacyClientNetworkHandler::$handle
    ),
    void,
    NetworkIdentifier const&    source,
    ContainerClosePacket const& packet
) {
    if (StackRequest::onContainerClose(packet)) return;
    origin(source, packet);
}

} // namespace stratum
