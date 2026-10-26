#pragma once

#include "bridge/RustBridge.h"

#include <ll/api/memory/Hook.h>
#include <mc/client/game/MinecraftGame.h>

namespace stratum {

// the game opens its pause screen from this handler when the window loses focus; while Rust says the library
// holds the file picker open, the handler is skipped
LL_TYPE_INSTANCE_HOOK(
    GameFocusLostHook,
    ll::memory::HookPriority::Normal,
    MinecraftGame,
    &MinecraftGame::$onAppFocusLost,
    void
) {
    StHandle* handle = RustBridge::handle();
    if (handle && st_pause_screen_unwanted(handle)) return;
    origin();
}

} // namespace stratum
