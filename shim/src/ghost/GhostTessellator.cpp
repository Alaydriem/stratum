#include "ghost/GhostTessellator.h"

#include "bridge/ModLog.h"
#include "ghost/BlockResolver.h"
#include "ghost/GhostGroupBuffer.h"

#include <mc/client/renderer/SupplementaryFieldAutoGenerationMode.h>
#include <mc/client/renderer/Tessellator.h>
#include <mc/client/renderer/block/BlockGraphics.h>
#include <mc/client/renderer/block/BlockTessellator.h>
#include <mc/client/renderer/block/BlockTessellatorCache.h>
#include <mc/client/world/level/biome/biome_color_sampling/TessellationPolicy.h>
#include <mc/deps/core/math/Color.h>
#include <mc/deps/core/string/HashedString.h>
#include <mc/deps/core_graphics/enums/PrimitiveMode.h>
#include <mc/world/level/BlockSource.h>
#include <mc/world/level/biome/biome_color_sampling/BiomeColorSampling.h>
#include <mc/world/level/block/Block.h>
#include <mc/world/level/block/BlockRenderLayer.h>
#include <mc/world/level/block/BlockType.h>
#include <mc/world/level/block/TintMethod.h>

#include <algorithm>
#include <cmath>
#include <cstring>
#include <string>

namespace stratum {

namespace {

constexpr uint16_t EMPTY          = 0xFFFF;
constexpr int      VERTS_PER_CELL = 24;
constexpr float    HALF           = 0.5f;
constexpr float    QUARTER        = 0.25f;
constexpr size_t   QUAD           = 4;
constexpr size_t   INDICES_PER_QUAD = 6;
constexpr float    MIN_NORMAL     = 1e-8f;
// each quad moves this far, in blocks, into its own block, so a real face over a ghost face always wins
constexpr float PUSH = 0.01f;
// any light value the cache holds is taken as computed, so the ghost is lit at full sky and block light
constexpr int FULL_LIGHT_BYTE = 0x0F;
// axis orders tried for the window index: the first axis is the slowest
constexpr int PERMS[6][3] = {
    {0, 1, 2},
    {0, 2, 1},
    {1, 0, 2},
    {1, 2, 0},
    {2, 0, 1},
    {2, 1, 0},
};
char const* const PERM_NAMES[6] = {"xyz", "xzy", "yxz", "yzx", "zxy", "zyx"};
// the probe cell inside the window; distinct values so no wrong axis order lands on the same index
constexpr int PROBE_X = 3;
constexpr int PROBE_Y = 6;
constexpr int PROBE_Z = 9;
// a quad faces up when its outward normal is within 60 degrees of +y
constexpr float UP_COS = 0.5f;

} // namespace

bool GhostTessellator::join(BlockSource& region, BlockResolver const& blocks, Tessellator& tessellator, BlockPos const& around) {
    leave();
    // a Block pointer is valid for one world only, and the tessellator keeps some (its local registry, the cache's air
    // block, its mesh caches), so each world gets a new instance of about 300 KB. The old one is pointed at the live
    // region first, so its destructor reads no freed BlockSource
    if (mTessellator) {
        mTessellator->setRegion(region);
        delete mTessellator;
    }
    mTessellator = new BlockTessellator(&region);
    mRegion      = &region;
    mTessellator->mUseCachedCalculation  = true;
    mTessellator->mApplyAmbientOcclusion = false;
    mTessellator->mUseNormals            = false;
    Block const* grass = Block::tryGetFromRegistry(HashedString("minecraft:grass_block")).as_ptr();
    mGrassBlock        = grass ? &grass->getBlockType() : nullptr;
    probeCache(blocks, around);
    if (mPerm >= 0) calibrateWinding(tessellator, blocks, BlockPos(around.x, around.y + 2, around.z));
    return ready();
}

// touches only this object's own fields: the tessellator still points at the old region, which may be freed by now,
// and is pointed at the next one by the next join
void GhostTessellator::leave() {
    mGrassBlock = nullptr;
    mRegion     = nullptr;
    mPerm    = -1;
    mOffset  = 0;
    mWinding = 1.0f;
}

int GhostTessellator::indexFor(int perm, int dx, int dy, int dz) {
    int const d[3] = {dx, dy, dz};
    return (d[PERMS[perm][0]] * WINDOW + d[PERMS[perm][1]]) * WINDOW + d[PERMS[perm][2]];
}

int GhostTessellator::index(int dx, int dy, int dz) const { return indexFor(std::max(mPerm, 0), dx, dy, dz); }

// sets one window entry to stone, asks BlockTessellatorCache::getBlock for the position each layout maps it to, and
// keeps the layout that answers stone
void GhostTessellator::probeCache(BlockResolver const& blocks, BlockPos const& around) {
    mPerm   = -1;
    mOffset = 0;
    BlockPos window(around.x - BORDER, around.y - BORDER, around.z - BORDER);
    fillWindow(nullptr, blocks, around, WINDOW - 2 * BORDER, WINDOW - 2 * BORDER, WINDOW - 2 * BORDER);
    BlockTessellatorCache& cache  = mTessellator->mBlockCache.get();
    auto&                  entries = cache.mBlocks.get();
    BlockPos               query(window.x + PROBE_X, window.y + PROBE_Y, window.z + PROBE_Z);
    for (int offset : {0, BORDER}) {
        for (int perm = 0; perm < 6 && mPerm < 0; ++perm) {
            cache.mPos       = BlockPos(window.x + offset, window.y + offset, window.z + offset);
            int i            = indexFor(perm, PROBE_X, PROBE_Y, PROBE_Z);
            entries[i]       = blocks.stone();
            Block const* got = &cache.getBlock(query);
            entries[i]       = blocks.air();
            if (got == blocks.stone()) {
                mPerm   = perm;
                mOffset = offset;
            }
        }
    }
    if (mPerm < 0) {
        ModLog::error("ghost cache window layout not found, the ghost is not built in this world", "{}");
        return;
    }
    ModLog::info(
        "ghost cache window layout",
        "{\"order\":" + ModLog::quote(PERM_NAMES[mPerm]) + ",\"offset\":" + std::to_string(mOffset) + "}"
    );
}

// fills the part of the window that a box of `sx` x `sy` x `sz` cells at `min` and its 2-cell border cover: from the
// job's padded grid where it reaches, air elsewhere, or air everywhere when `job` is null. The rest of the window is
// not read
void GhostTessellator::fillWindow(StGhostJob const* job, BlockResolver const& blocks, BlockPos const& min, int sx, int sy, int sz) {
    BlockTessellatorCache& cache = mTessellator->mBlockCache.get();
    BlockPos               window(min.x - BORDER, min.y - BORDER, min.z - BORDER);
    cache.mPos    = BlockPos(window.x + mOffset, window.y + mOffset, window.z + mOffset);
    cache.mRegion = mRegion;
    cache.mLocalRegistry->mAirBlock = blocks.air();
    std::memset(static_cast<void*>(&cache.mLightColors), FULL_LIGHT_BYTE, sizeof(cache.mLightColors));
    auto& cells    = cache.mBlocks.get();
    auto& extra    = cache.mExtraBlocks.get();
    auto& graphics = cache.mBlockGraphics[0].get();
    auto& extraGfx = cache.mBlockGraphics[1].get();
    int   wx       = std::min(sx + 2 * BORDER, WINDOW);
    int   wy       = std::min(sy + 2 * BORDER, WINDOW);
    int   wz       = std::min(sz + 2 * BORDER, WINDOW);
    // the padded grid starts one cell inside the window's corner and is dims + 2 cells on each axis
    int px = job ? static_cast<int>(job->dims[0]) + 2 : 0;
    int py = job ? static_cast<int>(job->dims[1]) + 2 : 0;
    int pz = job ? static_cast<int>(job->dims[2]) + 2 : 0;
    for (int dx = 0; dx < wx; ++dx) {
        for (int dy = 0; dy < wy; ++dy) {
            for (int dz = 0; dz < wz; ++dz) {
                int                  gx    = dx - 1;
                int                  gy    = dy - 1;
                int                  gz    = dz - 1;
                Block const*         block = blocks.air();
                BlockGraphics const* gfx   = blocks.airGraphics();
                if (job && gx >= 0 && gy >= 0 && gz >= 0 && gx < px && gy < py && gz < pz) {
                    uint16_t id = job->cells[(static_cast<size_t>(gy) * pz + gz) * px + gx];
                    if (Block const* ghost = blocks.block(id)) {
                        block = ghost;
                        gfx   = blocks.graphics(id);
                    }
                }
                int i       = index(dx, dy, dz);
                cells[i]    = block;
                extra[i]    = blocks.air();
                graphics[i] = gfx;
                extraGfx[i] = blocks.airGraphics();
            }
        }
    }
}

// the vertices from `first` on belong to one block. The tessellator leaves the cache path's tinted blocks grey, so a
// block with a tint method whose every vertex is grey has its RGB multiplied by the game's tessellation policy colour
// at its world position; a full grass block takes it on its up-facing quads only. A block the tessellator did tint
// is left as it is
void GhostTessellator::tint(mce::MeshData& data, size_t first, Block const& block, BlockPos const& pos) const {
    BlockType const& type   = block.getBlockType();
    TintMethod       method = type.mTintMethod;
    if (method == TintMethod::None) return;
    auto&  positions = data.mPositions.get();
    auto&  colors    = data.mColors.get();
    size_t last      = positions.size();
    if (colors.size() != last || last <= first) return;
    for (size_t i = first; i < last; ++i) {
        uint32_t c = colors[i];
        if ((c & 0xFFu) != ((c >> 8) & 0xFFu) || (c & 0xFFu) != ((c >> 16) & 0xFFu)) return;
    }
    bool top = &type == mGrassBlock;
    if (top && (last - first) % QUAD != 0) return;
    mce::Color colour = BiomeColorSampling::getTessellationPolicy(method).get(block, *mRegion, pos, nullptr);
    float      m[3]   = {
        std::clamp(colour.r, 0.0f, 1.0f),
        std::clamp(colour.g, 0.0f, 1.0f),
        std::clamp(colour.b, 0.0f, 1.0f),
    };
    for (size_t q = first; q < last; q += QUAD) {
        size_t end = std::min(q + QUAD, last);
        if (top) {
            glm::vec3 n   = glm::cross(positions[q + 1] - positions[q], positions[q + 2] - positions[q]) * mWinding;
            float     len = glm::length(n);
            if (len < MIN_NORMAL || n.y < UP_COS * len) continue;
        }
        for (size_t v = q; v < end; ++v) {
            uint32_t c   = colors[v];
            uint32_t out = c & 0xFF000000u;
            for (int ch = 0; ch < 3; ++ch) {
                float    byte = static_cast<float>((c >> (8 * ch)) & 0xFFu);
                uint32_t mul  = static_cast<uint32_t>(std::lround(byte * m[ch]));
                out |= std::min(mul, 0xFFu) << (8 * ch);
            }
            colors[v] = out;
        }
    }
}

// the vertices from `first` on belong to one block: each quad pushed inward along its normal, then the alpha. A
// block whose vertices are not whole quads keeps its geometry
void GhostTessellator::post(mce::MeshData& data, size_t first, uint8_t alpha) const {
    auto&  positions = data.mPositions.get();
    size_t last      = positions.size();
    if ((last - first) % QUAD == 0) {
        for (size_t q = first; q < last; q += QUAD) {
            glm::vec3 n   = glm::cross(positions[q + 1] - positions[q], positions[q + 2] - positions[q]);
            float     len = glm::length(n);
            if (len < MIN_NORMAL) continue;
            glm::vec3 shift = n * (-mWinding * PUSH / len);
            for (size_t v = q; v < q + QUAD; ++v) positions[v] += shift;
        }
    }
    auto& colors = data.mColors.get();
    if (colors.size() != last) return;
    // the alpha sits in the high byte of the packed ABGR colour
    uint32_t value = static_cast<uint32_t>(alpha) << 24;
    for (size_t i = first; i < last; ++i) colors[i] = (colors[i] & 0x00FFFFFFu) | value;
}

void GhostTessellator::calibrateWinding(Tessellator& tessellator, BlockResolver const& blocks, BlockPos const& pos) {
    if (tessellator.mTessellating || !mTessellator) return;
    fillWindow(nullptr, blocks, pos, 1, 1, 1);
    mce::MeshData& data = tessellator.mMeshData.get();
    tessellator.begin(Tessellator::DebugContextCallback{}, mce::PrimitiveMode::QuadList, 0, false);
    mTessellator->tessellateInWorld(tessellator, *blocks.stone(), pos, true);
    auto const& positions = data.mPositions.get();
    glm::vec3   centre(float(pos.x) + HALF, float(pos.y) + HALF, float(pos.z) + HALF);
    int         out = 0;
    int         in  = 0;
    for (size_t q = 0; q + QUAD <= positions.size(); q += QUAD) {
        glm::vec3 n    = glm::cross(positions[q + 1] - positions[q], positions[q + 2] - positions[q]);
        glm::vec3 face = (positions[q] + positions[q + 1] + positions[q + 2] + positions[q + 3]) * QUARTER - centre;
        float     d    = glm::dot(n, face);
        if (d > 0.0f) ++out;
        if (d < 0.0f) ++in;
    }
    // the probe mesh is dropped at once
    mce::Mesh discard =
        tessellator.end(Tessellator::UploadMode::Never, "stratum_ghost_winding", SupplementaryFieldAutoGenerationMode{});
    mWinding = in > out ? -1.0f : 1.0f;
    ModLog::info(
        "ghost winding",
        "{\"cross_points_out\":" + std::to_string(out) + ",\"cross_points_in\":" + std::to_string(in)
            + ",\"winding\":" + std::to_string(mWinding) + "}"
    );
}

// fills the window and tessellates every drawn cell of `job` into the game's tessellator, in world positions; false,
// with nothing begun, when the job does not fit one window
bool GhostTessellator::tessellate(Tessellator& tessellator, StGhostJob const& job, BlockResolver const& blocks, uint8_t alpha, Built& built) {
    int sx = static_cast<int>(job.dims[0]);
    int sy = static_cast<int>(job.dims[1]);
    int sz = static_cast<int>(job.dims[2]);
    bool fits = sx > 0 && sy > 0 && sz > 0 && sx + 2 * BORDER <= WINDOW && sy + 2 * BORDER <= WINDOW
             && sz + 2 * BORDER <= WINDOW
             && job.cell_count == static_cast<size_t>(sx + 2) * static_cast<size_t>(sy + 2) * static_cast<size_t>(sz + 2);
    if (!ready() || !fits) return false;
    BlockPos origin(job.origin[0], job.origin[1], job.origin[2]);
    fillWindow(&job, blocks, origin, sx, sy, sz);
    int px = sx + 2;
    int pz = sz + 2;
    size_t cells = 0;
    for (size_t i = 0; i < job.cell_count; ++i) cells += blocks.block(job.cells[i]) ? 1 : 0;

    BlockTessellator& bt   = *mTessellator;
    mce::MeshData&    data = tessellator.mMeshData.get();
    tessellator.begin(Tessellator::DebugContextCallback{}, mce::PrimitiveMode::QuadList, static_cast<int>(cells) * VERTS_PER_CELL, false);
    for (int y = 0; y < sy; ++y) {
        for (int z = 0; z < sz; ++z) {
            for (int x = 0; x < sx; ++x) {
                uint16_t     id    = job.cells[(static_cast<size_t>(y + 1) * pz + (z + 1)) * px + (x + 1)];
                Block const* block = id == EMPTY ? nullptr : blocks.block(id);
                if (!block || block == blocks.air()) continue;
                BlockPos pos(origin.x + x, origin.y + y, origin.z + z);
                // the world already holds a block of this name here. The cell stays in the cache window, so its
                // neighbours still connect to it
                if (job.skip_matching && &mRegion->getBlock(pos).getBlockType() == &block->getBlockType()) {
                    ++built.skipped;
                    continue;
                }
                ++built.cells;
                bt.mRenderingLayer = blocks.layer(id);
                size_t first       = data.mPositions->size();
                bt.tessellateInWorld(tessellator, *block, pos, true);
                tint(data, first, *block, pos);
                post(data, first, alpha);
            }
        }
    }
    built.vertices = data.mPositions->size();
    return true;
}

GhostTessellator::Built GhostTessellator::build(
    Tessellator&              tessellator,
    StGhostJob const&         job,
    BlockResolver const&      blocks,
    uint8_t                   alpha,
    std::optional<mce::Mesh>& out
) {
    Built built;
    if (tessellator.mTessellating) {
        built.busy = true;
        return built;
    }
    out.reset();
    if (!tessellate(tessellator, job, blocks, alpha, built)) return built;
    glm::vec3 shift(float(job.origin[0]), float(job.origin[1]), float(job.origin[2]));
    for (glm::vec3& position : tessellator.mMeshData->mPositions.get()) position -= shift;
    out.emplace(tessellator.end(Tessellator::UploadMode::Buffered, "stratum_ghost", SupplementaryFieldAutoGenerationMode{}));
    built.ok = built.vertices == 0 || out->isValid();
    // a job whose every cell was empty or skipped draws nothing
    if (built.vertices == 0 || !built.ok) out.reset();
    return built;
}

GhostTessellator::Built GhostTessellator::append(
    Tessellator&         tessellator,
    StGhostJob const&    job,
    BlockResolver const& blocks,
    uint8_t              alpha,
    GhostGroupBuffer&    group
) {
    Built built;
    if (tessellator.mTessellating) {
        built.busy = true;
        return built;
    }
    if (!tessellate(tessellator, job, blocks, alpha, built)) return built;
    mce::MeshData& data = tessellator.mMeshData.get();
    glm::vec3      shift(
        float(job.origin[0] - job.group_offset[0]),
        float(job.origin[1] - job.group_offset[1]),
        float(job.origin[2] - job.group_offset[2])
    );
    auto& positions = data.mPositions.get();
    glm::vec3 lo(0.0f);
    glm::vec3 hi(0.0f);
    for (size_t i = 0; i < positions.size(); ++i) {
        positions[i] -= shift;
        lo = i == 0 ? positions[i] : glm::min(lo, positions[i]);
        hi = i == 0 ? positions[i] : glm::max(hi, positions[i]);
    }
    data.mAABB.get() = {lo, hi};
    built.ok = group.add(data);
    // the copy is made, so the game's tessellator is free again in this frame
    tessellator.clear();
    // end leaves the tessellator free whatever clear leaves; the empty mesh is dropped at once
    if (tessellator.mTessellating) {
        mce::Mesh discard =
            tessellator.end(Tessellator::UploadMode::Never, "stratum_ghost_group_part", SupplementaryFieldAutoGenerationMode{});
    }
    return built;
}

GhostTessellator::Finished GhostTessellator::finish(Tessellator& tessellator, mce::MeshData& data, std::optional<mce::Mesh>& out) {
    Finished finished;
    if (tessellator.mTessellating) {
        finished.busy = true;
        return finished;
    }
    out.reset();
    auto&  positions = data.mPositions.get();
    size_t quads     = positions.size() / QUAD;
    finished.vertices = positions.size();
    // each quad as two triangles that keep its vertex order, so they face the way the quad-list meshes face
    auto& indices = data.mIndices.get();
    indices.resize(quads * INDICES_PER_QUAD);
    for (size_t q = 0; q < quads; ++q) {
        auto     v = static_cast<uint32_t>(q * QUAD);
        uint32_t* i = indices.data() + q * INDICES_PER_QUAD;
        i[0] = v;
        i[1] = v + 1;
        i[2] = v + 2;
        i[3] = v;
        i[4] = v + 2;
        i[5] = v + 3;
    }
    finished.indices = indices.size();
    data.mMode       = mce::PrimitiveMode::TriangleList;
    // a group whose every cell was empty or skipped draws nothing
    if (finished.indices == 0) {
        finished.ok = true;
        return finished;
    }
    // the game's own end path, as for every other ghost mesh: the merged vertices take the place of what a
    // tessellation would have left, with the vertex count it would have counted
    tessellator.begin(Tessellator::DebugContextCallback{}, mce::PrimitiveMode::TriangleList, 0, false);
    tessellator.mMeshData = std::move(data);
    tessellator.mCount    = static_cast<uint32_t>(finished.vertices);
    out.emplace(tessellator.end(Tessellator::UploadMode::Buffered, "stratum_ghost_group", SupplementaryFieldAutoGenerationMode{}));
    finished.ok = out->isValid();
    if (!finished.ok) out.reset();
    return finished;
}

} // namespace stratum
