#pragma once

#include "stratum.h"

class HudScreenController;
class ScreenController;
class IClientInstance;
class LocalPlayer;

namespace stratum {

// carries out the command st_use_decide gave: selects a hotbar slot through the HUD, places a block through the
// game mode, or hands a swap or a pick to StackRequest. It decides nothing; a command it cannot carry out (no HUD
// to select with, a select the game did not take, a build the game refused) places nothing, and a Place or Select
// and Place that placed nothing is reported to core with st_use_not_sent. The HUD controller is held from its tick
// until it is destroyed or the world is left
class UseExecutor {
public:
    static void run(IClientInstance& client, LocalPlayer& player, StUseCommand const& command);

    // the HUD screen's tick: the controller the hotbar select goes through
    static void captureHud(HudScreenController& hud);

    // a screen controller is being destroyed; when it is the captured HUD, the HUD is forgotten
    static void forgetHud(ScreenController const& controller);

    // forgets the HUD and ends the own-build guard; on mod disable and world leave
    static void reset();

    // true while the executor's own GameMode::buildBlock call runs, so the build hook lets it through
    static bool ownBuild() { return sOwnBuild; }

    // the isSimTick the game passed to its own last buildBlock call; the executor's call passes the same
    static void noteGameBuild(bool isSimTick) { sSimTick = isSimTick; }

private:
    static constexpr uint8_t KIND_SELECT = 2;
    static constexpr uint8_t KIND_SWAP   = 3;
    static constexpr uint8_t KIND_PICK   = 4;
    static constexpr uint8_t KIND_PLACE  = 5;
    // a HUD controller not ticked for this long is not used
    static constexpr double  HUD_FRESH_SECONDS = 0.5;

    static inline HudScreenController* sHud     = nullptr;
    static inline double               sHudSeen = 0.0;
    static inline bool                 sOwnBuild = false;
    static inline bool                 sSimTick  = false;
    // the no-HUD warning is logged once until a HUD is captured again
    static inline bool                 sNoHudLogged = false;

    static bool select(LocalPlayer& player, uint8_t slot);
    static bool place(LocalPlayer& player, StUseCommand const& command);
    static void notSent(StUseCommand const& command);
};

} // namespace stratum
