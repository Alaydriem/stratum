#pragma once

#include "ghost/BlockResolver.h"
#include "ghost/GhostGroupBuffer.h"
#include "ghost/GhostTessellator.h"
#include "stratum.h"

#include <mc/deps/minecraft_renderer/renderer/Mesh.h>
#include <mc/deps/minecraft_renderer/renderer/TexturePtr.h>
#include <mc/deps/minecraft_renderer/resources/ClientTexture.h>
#include <mc/deps/minecraft_renderer/resources/ServerTexture.h>

#include <array>
#include <cstdint>
#include <optional>
#include <variant>
#include <vector>

class BlockSource;
class LevelRenderer;
class ScreenContext;
struct FrameRenderObject;
namespace mce { class MaterialPtr; }

namespace stratum {

// the ghost on the render thread: each frame it takes the frame core published, frees and builds the mesh slots it
// lists, draws the listed slots that are in view see-through over the world, farthest first, and reports what it
// built and how many it left out of view. What to build and draw is core's; the shim holds only the slot table and
// its per-slot scratch, sized once to core's slot capacity, and the palette's block lookups
class GhostRenderer {
public:
    // installs the render and block-change hooks
    static void start();

    // removes the hooks; the meshes stay until the game frees its device, because they belong to it
    static void stop();

    // after LevelRenderer::renderLevel's own drawing: take, apply, draw, report
    static void afterRenderLevel(LevelRenderer& renderer, ScreenContext& screen, FrameRenderObject const& frame);

    // the game block of palette id `id` in the world the ghost is drawn in; null outside a world or for an id that
    // did not resolve. The client tick may call it, because the render and the tick run on the same thread
    [[nodiscard]] static Block const* paletteBlock(uint16_t id);

private:
    using TextureVariant = std::variant<std::monostate, mce::TexturePtr, mce::ClientTexture, mce::ServerTexture>;

    // the world the slots, the blocks and the tessellator belong to; null outside a world
    BlockSource*                          mRegion = nullptr;
    bool                                  mUsable = false;
    BlockResolver                         mBlocks;
    GhostTessellator                      mTessellator;
    std::vector<std::optional<mce::Mesh>> mSlots;
    // the mini ghost group being built and each slot's last finished group; sized with the slot table
    GhostGroupBuffer                      mGroup;
    // the generation the group buffer belongs to
    uint64_t                              mGeneration = 0;
    // one entry per job of a frame, at most one per slot; reserved with the slot table
    std::vector<StSlotBuilt>      mBuilt;
    std::optional<TextureVariant> mAtlas;
    // per slot, sized with the slot table: the built box in cells, and its squared distance this frame
    std::vector<std::array<uint32_t, 3>> mSizes;
    std::vector<float>                   mDistance;
    // this frame's draw indices in view, farthest first; sized with the slot table
    std::vector<uint32_t>         mOrder;
    uint32_t                      mCulled      = 0;
    uint32_t                      mMeshesDrawn = 0;
    // set on a join, cleared once a report has carried it
    bool                          mWorldReset = false;
    // core's world epoch the slots, the blocks and the tessellator belong to
    uint64_t                      mWorldEpoch = 0;
    // this frame resolved a palette; its fallback counts go in the report
    bool                          mPaletteResolved = false;
    mce::MaterialPtr const*       mBlend   = nullptr;
    // this frame's build time, the sum of its jobs
    uint64_t                      mBuildUs = 0;
    // this frame's built cells and skipped cells over the jobs it ran
    uint32_t                      mCellsBuilt   = 0;
    uint32_t                      mSkippedCells = 0;

    static GhostRenderer* gInstance;

    void frameIn(LevelRenderer& renderer, ScreenContext& screen, FrameRenderObject const& frame);
    void join(LevelRenderer& renderer, ScreenContext& screen, BlockSource& region, bool askReset);
    void leave();
    void apply(StGhostFrame const& frame, ScreenContext& screen);
    void resolvePalette(StGhostFrame const& frame);
    void build(StGhostFrame const& frame, ScreenContext& screen);
    bool buildReplace(StGhostJob const& job, ScreenContext& screen, uint8_t alpha);
    bool buildGroup(StGhostJob const& job, ScreenContext& screen, uint8_t alpha);
    uint64_t draw(StGhostFrame const& frame, ScreenContext& screen, float const* camera);
    uint32_t cull(StGhostFrame const& frame, ScreenContext& screen, float const* camera);
    void drawMesh(ScreenContext& screen, mce::Mesh const& mesh, mce::MaterialPtr const& material,
                  TextureVariant const& texture, float const* origin, int32_t const* offset, float scale,
                  float const* camera);
    static void setConstants(ScreenContext& screen);
};

} // namespace stratum
