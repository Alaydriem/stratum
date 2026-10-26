#include "ghost/BlockResolver.h"

#include "bridge/ModLog.h"

#include <mc/client/renderer/block/BlockGraphics.h>
#include <mc/deps/core/string/HashedString.h>
#include <mc/deps/nbt/CompoundTag.h>
#include <mc/deps/nbt/StringTag.h>
#include <mc/util/Mirror.h>
#include <mc/util/Rotation.h>
#include <mc/world/level/block/Block.h>
#include <mc/world/level/block/states/VanillaBlockStateTransformUtils.h>

#include <string>
#include <string_view>

namespace stratum {

namespace {

constexpr uint16_t EMPTY = 0xFFFF;

} // namespace

bool BlockResolver::join() {
    leave();
    mAir         = Block::tryGetFromRegistry(HashedString("minecraft:air")).as_ptr();
    mStone       = Block::tryGetFromRegistry(HashedString("minecraft:stone")).as_ptr();
    mUnknown     = Block::tryGetFromRegistry(HashedString("minecraft:unknown")).as_ptr();
    mAirGraphics = mAir ? BlockGraphics::getForBlock(*mAir) : nullptr;
    ModLog::info(
        "ghost blocks joined the world",
        std::string("{\"air\":") + (mAir ? "true" : "false") + ",\"stone\":" + (mStone ? "true" : "false")
            + ",\"unknown\":" + (mUnknown ? "true" : "false") + "}"
    );
    return mAir && mStone;
}

void BlockResolver::leave() {
    mBlocks.clear();
    mGraphics.clear();
    mLayers.clear();
    mAir         = nullptr;
    mAirGraphics = nullptr;
    mStone       = nullptr;
    mUnknown     = nullptr;
}

void BlockResolver::resolve(StGhostBlock const* entries, uint32_t count, uint8_t rotation) {
    // sized by core's palette, which is bounded below u16::MAX entries
    mBlocks.assign(count, nullptr);
    mGraphics.assign(count, nullptr);
    mLayers.assign(count, 0);
    for (uint32_t id = 0; id < count; ++id) {
        Block const* block = turn(lookup(entries[id].nbt), rotation);
        if (!block) continue;
        BlockGraphics const* graphics = BlockGraphics::getForBlock(*block);
        mBlocks[id]                   = block;
        mGraphics[id]                 = graphics;
        mLayers[id]                   = graphics ? static_cast<int>(graphics->mRenderLayer) : 0;
    }
}

// the block turned by `rotation` quarter turns, clockwise from above; the lookup result is never turned twice
// because every resolve starts from the palette's own states. A null transform result keeps the block
Block const* BlockResolver::turn(Block const* block, uint8_t rotation) {
    if (!block || block == mUnknown) return block;
    Rotation quarter = Rotation::None;
    switch (rotation & 3) {
    case 1:
        quarter = Rotation::Rotate90;
        break;
    case 2:
        quarter = Rotation::Rotate180;
        break;
    case 3:
        quarter = Rotation::Rotate270;
        break;
    default:
        return block;
    }
    if (Block const* turned = VanillaBlockStateTransformUtils::transformBlock(*block, quarter, Mirror::None)) block = turned;
    if (VanillaBlockStateTransformUtils::hasTransformableConnectionStates(*block)) {
        if (Block const* turned = VanillaBlockStateTransformUtils::transformConnectionStates(*block, quarter, Mirror::None))
            block = turned;
    }
    if (VanillaBlockStateTransformUtils::hasTransformableWallStates(*block)) {
        if (Block const* turned = VanillaBlockStateTransformUtils::transformWallConnectionTypes(*block, quarter, Mirror::None))
            block = turned;
    }
    return block;
}

// name and states first, then the name with its default states, then the game's unknown block
Block const* BlockResolver::lookup(StBytes const& nbt) {
    if (nbt.len == 0) {
        ++mUnknownCount;
        return mUnknown;
    }
    auto tag = CompoundTag::fromBinaryNbt(std::string_view(reinterpret_cast<char const*>(nbt.ptr), nbt.len), true);
    if (tag) {
        if (Block const* block = Block::tryGetFromRegistry(*tag).as_ptr()) return block;
        if (tag->contains("name", Tag::String)) {
            std::string const& name  = tag->at("name").get<StringTag>();
            Block const*       block = Block::tryGetFromRegistry(HashedString(name)).as_ptr();
            if (block) {
                ++mDefaultStateCount;
                return block;
            }
        }
    }
    ++mUnknownCount;
    return mUnknown;
}

Block const* BlockResolver::block(uint16_t id) const {
    if (id == EMPTY || id >= mBlocks.size()) return nullptr;
    return mBlocks[id];
}

uint32_t BlockResolver::takeUnknown() {
    uint32_t count = mUnknownCount;
    mUnknownCount  = 0;
    return count;
}

uint32_t BlockResolver::takeDefaultState() {
    uint32_t count     = mDefaultStateCount;
    mDefaultStateCount = 0;
    return count;
}

} // namespace stratum
