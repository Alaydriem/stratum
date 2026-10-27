#pragma once

#include "bridge/RustBridge.h"
#include "state/FrameStatusReader.h"

#include <ll/api/memory/Hook.h>
#include <mc/client/renderer/game/LevelRenderer.h>
#include <mc/world/level/BlockSource.h>
#include <mc/world/level/block/Block.h>

namespace stratum {

// the level renderer's block-changed listener, which the game calls on the game thread for every block change the
// client sees (placed, broken or sent by the server) so it can rebuild its chunk meshes; each position goes to Rust,
// which rebuilds the ghost section that holds it; the block on layer 0 also goes to Rust, which checks the
// placements that wait for the server against it
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
        // the placed block is on layer 0; layer 1 holds the water of a waterlogged block
        if (layer == 0) st_block_seen(
            handle,
            blockPosition.x,
            blockPosition.y,
            blockPosition.z,
            RustBridge::view(block.getTypeName()),
            FrameStatusReader::nowSeconds()
        );
    }
}

} // namespace stratum
