#pragma once

#include "stratum.h"

#include <atomic>
#include <cstdint>

class IClientInstance;
class Player;

namespace stratum {

// runs the use calls of a hold. While the use key is down and no menu shows, it makes one use call per
// GameMode::buildingTickDelay from the client tick (held repeats do not reach the game's build action), and one
// on the press when the tick has not made it yet. Each call reads the probe, asks Rust for a spot, reads the spot's
// neighbours and the inventory, asks Rust what to do and has UseExecutor carry it out. On key up it tells Rust once.
// It keeps only the hold flag, the hold's last decision kind, the time of the last call, the swap answer that call
// left, the local player of the hold and the status of the last call
class UseDriver {
public:
    // after the game's client update
    static void tick(IClientInstance& client);

    // ClientInputCallbacks::handleBuildAction, before the game: true cancels the game's handling of the press
    static bool press(IClientInstance& client);

    // GameMode::buildBlock, before the game: true skips the game's own build for the local player, because the
    // hold's last decision was not to leave the use to the game. The executor's own build always goes through, and
    // so does every build of another player's game mode, such as the local world's server side
    static bool blocksGameBuild(Player const& player, bool isSimTick);

    // ends the hold without telling Rust and clears the executor and the request in flight; after the hooks are
    // removed on mod disable
    static void reset();

private:
    static constexpr uint8_t KIND_VANILLA = 0;

    static inline bool    sHolding    = false;
    static inline uint8_t sLastKind   = KIND_VANILLA;
    static inline double  sLastCall   = 0.0;
    // the swap answer as the last call left it
    static inline uint8_t sLastAnswer = 0;
    // the local player of the last use call, compared by address only; the build hook also runs on the local
    // world's server thread, so it is atomic
    static inline std::atomic<Player const*> sPlayer{nullptr};
    // the last status of the use calls, so a failure and a recovery are each logged once
    static inline StStatus sLastStatus = ST_STATUS_OK;

    // the use key is down and no menu shows
    static bool keyHeld(IClientInstance& client);

    // a call is due once the build delay has passed since the last call, or as soon as the answer to a swap or
    // pick has come, so the block is placed on the server's answer
    static bool due(IClientInstance& client, double now);

    // one use call; returns the command's kind
    static uint8_t call(IClientInstance& client, double now, bool pressed);

    static void release();
};

} // namespace stratum
