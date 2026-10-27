#pragma once

#include "stratum.h"

class BlockSource;
class GameMode;
class LocalPlayer;

namespace stratum {

// reads the player's view for one use call: the eye, the view direction, the cells the player's box fills, the
// reach, the game mode and the hit. The
// hit is the first block on the view ray within reach that cannot be built over, so a ghost spot behind grass is
// not reached through a wall; with no such block it is the game's own hit when that is a block. Read-only, and it
// does not allocate. The time, the press flags and the swap answer are the caller's
class ProbeReader {
public:
    static void read(LocalPlayer& player, BlockSource& region, GameMode& gameMode, StUseProbe& probe);

private:
    // the most cell crossings one walk makes; a reach of 12 crosses at most 3 * 13 cells
    static constexpr int MAX_CROSSINGS = 64;

    static void readBody(LocalPlayer& player, StUseProbe& probe);
    static bool walk(BlockSource& region, StUseProbe& probe);
    static void gameHit(LocalPlayer& player, BlockSource& region, StUseProbe& probe);
};

} // namespace stratum
