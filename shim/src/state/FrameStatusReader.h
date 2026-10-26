#pragma once

#include "stratum.h"

#include <string>

class ClientInstance;

namespace stratum {

// reads the game state Rust needs for one tick; read-only, it never changes the game
class FrameStatusReader {
public:
    // with no active world only the time is set; in a world with no local player, has_player is false and the
    // world key is empty.
    // `status.world` views `worldStorage`, which the caller keeps alive and unchanged for the st_tick call
    static void read(ClientInstance& client, StFrameStatus& status, std::string& worldStorage);

    static double nowSeconds();
};

} // namespace stratum
