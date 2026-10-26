#pragma once

#include "stratum.h"

#include <mc/deps/minecraft_renderer/renderer/Mesh.h>
#include <mc/world/level/BlockPos.h>

#include <cstddef>
#include <cstdint>
#include <optional>

class Block;
class BlockSource;
class BlockTessellator;
class BlockType;
class Tessellator;
namespace mce { class MeshData; }

namespace stratum {

class BlockResolver;
class GhostGroupBuffer;

// builds one ghost job into an mce::Mesh, or into the mini ghost group buffer, with the game's BlockTessellator.
// Neighbours come from the job's padded grid through the tessellator's 20 x 20 x 20 block cache window, so ghost blocks connect to each other and not to
// the world; a tinted block the tessellator leaves grey takes the game's biome colour at its world position. Owns one BlockTessellator
// for each world, made again on every join; the last one is never deleted, like the renderer that holds it
class GhostTessellator {
public:
    // what one build gave
    struct Built {
        bool     ok       = false;
        // the game's tessellator was in use, so nothing was built and the job is not answered
        bool     busy     = false;
        uint64_t vertices = 0;
        // cells turned into geometry, and cells left out because the world already holds a block of that name
        uint32_t cells    = 0;
        uint32_t skipped  = 0;
    };

    // what one group upload gave
    struct Finished {
        bool     ok       = false;
        // the game's tessellator was in use, so nothing was uploaded and the job is not answered
        bool     busy     = false;
        uint64_t vertices = 0;
        uint64_t indices  = 0;
    };

    // makes a new tessellator for `region` in place of the last world's one, finds the cache window's layout and the
    // quad winding; false when the window layout is not the one the game was measured with, so nothing can be built
    // in this world
    bool join(BlockSource& region, BlockResolver const& blocks, Tessellator& tessellator, BlockPos const& around);

    // forgets the world that was left; the tessellator is replaced by the next join
    void leave();

    [[nodiscard]] bool ready() const { return mTessellator != nullptr && mPerm >= 0; }

    // tessellates every drawn cell of `job` into `out`, positions relative to the job's origin, each quad pushed
    // inward along its normal, with `alpha` in every vertex colour's alpha byte. `out` is empty when no cell was
    // drawn. A job larger than one section builds nothing and is not ok; a tessellator in use builds nothing, is
    // busy and leaves `out` as it was
    Built build(Tessellator& tessellator, StGhostJob const& job, BlockResolver const& blocks, uint8_t alpha,
                std::optional<mce::Mesh>& out);

    // tessellates `job` as `build` does, positions relative to the job's origin and then moved by its group offset,
    // and adds the vertices to `group` in place of making a mesh; the game's tessellator is cleared after the copy.
    // Not ok when the job does not fit or the group buffer refuses the vertices; busy as `build`
    Built append(Tessellator& tessellator, StGhostJob const& job, BlockResolver const& blocks, uint8_t alpha,
                 GhostGroupBuffer& group);

    // uploads the merged vertices in `data` into `out` as one triangle list with its own 32-bit index buffer, six
    // indices per quad. `data` is moved into the mesh; `out` is empty when `data` holds no whole quad. A tessellator
    // in use uploads nothing, is busy and leaves `data` and `out` as they were
    Finished finish(Tessellator& tessellator, mce::MeshData& data, std::optional<mce::Mesh>& out);

private:
    static constexpr int WINDOW = 20;
    static constexpr int BORDER = 2;

    BlockTessellator* mTessellator = nullptr;
    BlockSource*      mRegion      = nullptr;
    // axis order of the window index, an index into the permutation table; -1 when the probe found none
    int mPerm = -1;
    // how far mPos sits inside the window's minimum corner
    int mOffset = 0;
    // +1 when the cross product of a quad's first two edges points out of the block, -1 when it points in
    float mWinding = 1.0f;
    // the full grass block of this world, which takes the biome tint on its top face only
    BlockType const* mGrassBlock = nullptr;

    bool               tessellate(Tessellator& tessellator, StGhostJob const& job, BlockResolver const& blocks, uint8_t alpha,
                                  Built& built);
    void               probeCache(BlockResolver const& blocks, BlockPos const& around);
    void               calibrateWinding(Tessellator& tessellator, BlockResolver const& blocks, BlockPos const& pos);
    void               fillWindow(StGhostJob const* job, BlockResolver const& blocks, BlockPos const& min, int sx, int sy, int sz);
    [[nodiscard]] int  index(int dx, int dy, int dz) const;
    static int         indexFor(int perm, int dx, int dy, int dz);
    void               tint(mce::MeshData& data, size_t first, Block const& block, BlockPos const& pos) const;
    void               post(mce::MeshData& data, size_t first, uint8_t alpha) const;
};

} // namespace stratum
