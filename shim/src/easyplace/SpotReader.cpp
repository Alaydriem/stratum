#include "easyplace/SpotReader.h"

#include <mc/world/level/BlockPos.h>
#include <mc/world/level/BlockSource.h>
#include <mc/world/level/block/Block.h>
#include <mc/world/level/block/BlockType.h>

namespace stratum {

namespace {

// the step to the neighbour on each face, in the order StSpotWorld::against uses: Down, Up, North, South, West, East
constexpr int NORMALS[6][3] = {{0, -1, 0}, {0, 1, 0}, {0, 0, -1}, {0, 0, 1}, {-1, 0, 0}, {1, 0, 0}};

} // namespace

void SpotReader::read(BlockSource& region, int32_t const (&spot)[3], StSpotWorld& world) {
    for (int face = 0; face < 6; ++face) {
        BlockPos         pos(spot[0] + NORMALS[face][0], spot[1] + NORMALS[face][1], spot[2] + NORMALS[face][2]);
        Block const&     block = region.getBlock(pos);
        BlockType const& type  = block.getBlockType();
        world.against[face]    = !type.canBeBuiltOver(block, region, pos) && !type.isInteractiveBlock();
    }
}

} // namespace stratum
