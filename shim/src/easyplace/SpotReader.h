#pragma once

#include "stratum.h"

#include <cstdint>

class BlockSource;

namespace stratum {

// reads the six neighbours of the spot st_use_aim gave: a neighbour can be placed against when it cannot be built
// over and is not an interactive block (a door, a chest or a sign would open instead of taking the block).
// Read-only
class SpotReader {
public:
    static void read(BlockSource& region, int32_t const (&spot)[3], StSpotWorld& world);
};

} // namespace stratum
