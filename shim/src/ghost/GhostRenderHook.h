#pragma once

#include "ghost/GhostRenderer.h"

#include <ll/api/memory/Hook.h>
#include <mc/client/gui/screens/ScreenContext.h>
#include <mc/client/renderer/game/LevelRenderer.h>
#include <mc/deps/minecraft_renderer/objects/FrameRenderObject.h>

namespace stratum {

// the world render: the ghost is drawn after the game's own level pass, depth tested against it
LL_TYPE_INSTANCE_HOOK(
    GhostRenderHook,
    ll::memory::HookPriority::Normal,
    LevelRenderer,
    &LevelRenderer::renderLevel,
    void,
    ScreenContext&           screenContext,
    FrameRenderObject const& renderObj
) {
    origin(screenContext, renderObj);
    GhostRenderer::afterRenderLevel(*this, screenContext, renderObj);
}

} // namespace stratum
