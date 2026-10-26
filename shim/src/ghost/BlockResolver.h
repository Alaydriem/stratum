#pragma once

#include "stratum.h"

#include <cstdint>
#include <vector>

class Block;
class BlockGraphics;

namespace stratum {

// turns the ghost palette into the game's blocks for one world, because a Block pointer is valid only in the
// world it was looked up in. Each entry is read from its NBT bytes into a CompoundTag, then looked up by name and
// states, then by name with the default states, then as the game's unknown block; each fallback is counted for
// the next report. Holds one entry per palette id, sized by core's palette
class BlockResolver {
public:
    // looks up air, stone and the unknown block for a newly joined world and drops the palette; false when air or
    // stone is missing, which no ghost can be built without
    bool join();

    // drops every lookup; the pointers belong to the world that was left
    void leave();

    // resolves `count` palette entries in place of the last palette, each turned by `rotation` quarter turns
    // (0..3, clockwise seen from above). The entries are always the unturned states
    void resolve(StGhostBlock const* entries, uint32_t count, uint8_t rotation);

    // the block for palette id `id`; null for an empty cell, an id outside the palette or an entry that did not
    // resolve at all
    [[nodiscard]] Block const* block(uint16_t id) const;

    // the cached graphics and render layer of palette id `id`; only valid when block(id) is not null
    [[nodiscard]] BlockGraphics const* graphics(uint16_t id) const { return mGraphics[id]; }
    [[nodiscard]] int                  layer(uint16_t id) const { return mLayers[id]; }

    [[nodiscard]] Block const*         air() const { return mAir; }
    [[nodiscard]] BlockGraphics const* airGraphics() const { return mAirGraphics; }
    [[nodiscard]] Block const*         stone() const { return mStone; }

    // the fallbacks counted since the last call, for the report
    uint32_t takeUnknown();
    uint32_t takeDefaultState();

private:
    std::vector<Block const*>         mBlocks;
    std::vector<BlockGraphics const*> mGraphics;
    std::vector<int>                  mLayers;
    Block const*                      mAir         = nullptr;
    BlockGraphics const*              mAirGraphics = nullptr;
    Block const*                      mStone       = nullptr;
    Block const*                      mUnknown     = nullptr;
    uint32_t                          mUnknownCount      = 0;
    uint32_t                          mDefaultStateCount = 0;

    Block const* lookup(StBytes const& nbt);
    Block const* turn(Block const* block, uint8_t rotation);
};

} // namespace stratum
