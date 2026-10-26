#pragma once

#include "bridge/RustBridge.h"

#include <ll/api/memory/Hook.h>
#include <mc/client/renderer/game/LevelRenderer.h>
#include <mc/world/level/BlockSource.h>
#include <mc/world/level/block/Block.h>

namespace stratum {

// the level renderer's block-changed listener, which the game calls on the game thread for every block change the
// client sees (placed, broken or sent by the server) so it can rebuild its chunk meshes; each position goes to Rust,
// which rebuilds the ghost section that holds it
LL_TYPE_INSTANCE_HOOK(
    BlockChangeHook,
    ll::memory::HookPriority::Normal,
    LevelRenderer,
    &LevelRenderer::$onBlockChanged,
    void,
    BlockSource&                 source,
    BlockPos const&              blockPosition,
    uint                         layer,
    Block const&                 block,
    Block const&                 oldBlock,
    int                          updateFlags,
    ActorBlockSyncMessage const* syncMsg,
    BlockChangedEventTarget      eventTarget,
    Actor*                       blockChangeSource
) {
    origin(source, blockPosition, layer, block, oldBlock, updateFlags, syncMsg, eventTarget, blockChangeSource);
    if (StHandle* handle = RustBridge::handle()) {
        st_block_changed(handle, blockPosition.x, blockPosition.y, blockPosition.z);
    }
}

} // namespace stratum
