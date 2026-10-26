#include "ghost/GhostRenderer.h"

#include "bridge/ModLog.h"
#include "bridge/RustBridge.h"
#include "ghost/BlockChangeHook.h"
#include "ghost/GhostRenderHook.h"
#include "state/FrameStatusReader.h"

#include <mc/client/game/IClientInstance.h>
#include <mc/client/gui/screens/ScreenContext.h>
#include <mc/client/player/LocalPlayer.h>
#include <mc/client/renderer/ActorShaderManager.h>
#include <mc/client/renderer/Tessellator.h>
#include <mc/client/renderer/game/ItemInHandRenderer.h>
#include <mc/client/renderer/game/LevelRenderer.h>
#include <mc/deps/core/math/Color.h>
#include <mc/deps/core/math/Matrix.h>
#include <mc/deps/core/math/Vec2.h>
#include <mc/deps/core/math/Vec4.h>
#include <mc/deps/minecraft_renderer/constantbuffers/ActorConstants.h>
#include <mc/deps/minecraft_renderer/constantbuffers/GlobalConstantBuffers.h>
#include <mc/deps/minecraft_renderer/objects/FrameRenderObject.h>
#include <mc/deps/minecraft_renderer/objects/ViewRenderData.h>
#include <mc/deps/minecraft_renderer/objects/ViewRenderObject.h>
#include <mc/deps/minecraft_renderer/renderer/MaterialPtr.h>
#include <mc/deps/minecraft_renderer/resources/OffscreenCaptureDescription.h>
#include <mc/deps/renderer/Camera.h>
#include <mc/deps/renderer/MatrixStack.h>
#include <mc/world/level/BlockPos.h>
#include <mc/world/level/BlockSource.h>

#include <glm/gtc/matrix_transform.hpp>

#include <algorithm>
#include <array>
#include <cmath>
#include <string>
#include <string_view>

namespace stratum {

namespace {

constexpr double US_PER_SECOND = 1000000.0;
constexpr float  ALPHA_MAX     = 255.0f;
constexpr float  HALF          = 0.5f;
// what core counts a built mesh's GPU memory at: the measured vertex size on the GPU
constexpr uint64_t GPU_BYTES_PER_VERTEX = 20;
// a group mesh's own index buffer holds 32-bit indices
constexpr uint64_t INDEX_BYTES          = 4;
constexpr int      CORNERS              = 8;
constexpr int      PLANES               = 6;

} // namespace

GhostRenderer* GhostRenderer::gInstance = nullptr;

void GhostRenderer::start() {
    // never freed: the meshes it holds belong to the game's device, which is gone by the time the mod unloads
    if (!gInstance) gInstance = new GhostRenderer();
    GhostRenderHook::hook();
    BlockChangeHook::hook();
}

void GhostRenderer::stop() {
    BlockChangeHook::unhook();
    GhostRenderHook::unhook();
}

void GhostRenderer::afterRenderLevel(LevelRenderer& renderer, ScreenContext& screen, FrameRenderObject const& frame) {
    if (gInstance) gInstance->frameIn(renderer, screen, frame);
}

void GhostRenderer::frameIn(LevelRenderer& renderer, ScreenContext& screen, FrameRenderObject const& frame) {
    StHandle* handle = RustBridge::handle();
    if (!handle) return;
    IClientInstance& client = renderer.mClientInstance;
    BlockSource*     region = client.getRegion();
    if (!region || !client.getLocalPlayer()) {
        if (mRegion) leave();
        return;
    }
    StGhostFrame ghost{};
    if (st_ghost_take(handle, &ghost) != ST_STATUS_OK) return;
    // core's epoch is the one trigger of a rejoin: it moves once per world, also when the new BlockSource sits at the
    // old address, and core has already started the new generation then. While core is between two worlds the shim
    // holds nothing, so a new BlockSource seen early does not join. A BlockSource that changes under an unchanged
    // epoch is the safety net: the join then asks core for a new generation
    bool epochMoved = ghost.world_epoch != mWorldEpoch;
    mWorldEpoch     = ghost.world_epoch;
    if (!ghost.in_world) {
        if (mRegion) leave();
    } else if (epochMoved || region != mRegion) {
        if (mRegion) leave();
        join(renderer, screen, *region, !epochMoved);
    }
    apply(ghost, screen);
    float camera[3] = {0.0f, 0.0f, 0.0f};
    if (!frame.mViews->empty()) {
        glm::vec3 const& pos = *frame.mViews->front().mViewData->mCameraPos;
        camera[0]            = pos.x;
        camera[1]            = pos.y;
        camera[2]            = pos.z;
    }
    uint64_t drawUs = draw(ghost, screen, camera);
    StGhostReport report{};
    report.built                = mBuilt.data();
    report.built_count          = static_cast<uint32_t>(mBuilt.size());
    report.unknown_blocks       = mBlocks.takeUnknown();
    report.default_state_blocks = mBlocks.takeDefaultState();
    report.palette_resolved     = mPaletteResolved;
    report.build_us             = mBuildUs;
    report.draw_us_sum          = drawUs;
    report.draw_frames          = 1;
    report.draw_us_max          = drawUs;
    report.meshes_drawn         = mMeshesDrawn;
    report.cells_built          = mCellsBuilt;
    report.skipped_cells        = mSkippedCells;
    report.culled               = mCulled;
    report.world_reset          = mWorldReset;
    if (st_ghost_report(handle, &report) == ST_STATUS_OK) mWorldReset = false;
}

void GhostRenderer::join(LevelRenderer& renderer, ScreenContext& screen, BlockSource& region, bool askReset) {
    IClientInstance& client = renderer.mClientInstance;
    mRegion                 = &region;
    // the meshes and block lookups are gone; unless core started a new generation for this join already, it starts
    // one with the palette and a clear
    mWorldReset             = mWorldReset || askReset;
    BlockPos feet           = client.getLocalPlayer()->getFeetBlockPos();
    mUsable = mBlocks.join() && mTessellator.join(region, mBlocks, screen.tessellator, feet);
    ItemInHandRenderer* hand = client.getItemInHandRenderer();
    mBlend                   = hand ? &hand->mMatBlendBlock.get() : nullptr;
    mAtlas.emplace(std::in_place_type<mce::TexturePtr>, renderer.mAtlasTexture.get());
    ModLog::info(
        "ghost renderer joined the world",
        std::string("{\"usable\":") + (mUsable ? "true" : "false") + ",\"blend_material\":"
            + (mBlend ? "true" : "false") + "}"
    );
}

// a Block pointer and the tessellator's region belong to the world that was left, so both go with the meshes
void GhostRenderer::leave() {
    for (auto& slot : mSlots) slot.reset();
    mGroup.clear();
    mTessellator.leave();
    mBlocks.leave();
    mAtlas.reset();
    mBlend  = nullptr;
    mRegion = nullptr;
    mUsable = false;
}

// in core's order: free every slot, then the frees, then the palette, then the jobs
void GhostRenderer::apply(StGhostFrame const& frame, ScreenContext& screen) {
    if (frame.slot_capacity != mSlots.size()) {
        // sized once to core's slot capacity, which changes only with the config
        mSlots.clear();
        mSlots.resize(frame.slot_capacity);
        mBuilt.clear();
        mBuilt.reserve(frame.slot_capacity);
        mSizes.assign(frame.slot_capacity, {0, 0, 0});
        mDistance.assign(frame.slot_capacity, 0.0f);
        mOrder.assign(frame.slot_capacity, 0);
        mGroup.resize(frame.slot_capacity);
    }
    // a group build of an earlier generation is never finished
    if (frame.clear_all || frame.generation != mGeneration) mGroup.clear();
    mGeneration = frame.generation;
    if (frame.clear_all) {
        for (auto& slot : mSlots) slot.reset();
    }
    for (uint32_t i = 0; i < frame.free_count; ++i) {
        if (frame.frees[i] >= mSlots.size()) continue;
        mSlots[frame.frees[i]].reset();
        mGroup.free(frame.frees[i]);
    }
    mPaletteResolved = frame.has_palette;
    if (frame.has_palette) resolvePalette(frame);
    build(frame, screen);
}

// the whole palette in this frame; the log line gives its cost, so a large palette's frame can be measured
void GhostRenderer::resolvePalette(StGhostFrame const& frame) {
    double start = FrameStatusReader::nowSeconds();
    mBlocks.resolve(frame.palette, frame.palette_count, frame.rotation);
    auto us = static_cast<uint64_t>((FrameStatusReader::nowSeconds() - start) * US_PER_SECOND);
    ModLog::info(
        "ghost palette resolved",
        std::string("{\"entries\":") + std::to_string(frame.palette_count) + ",\"us\":" + std::to_string(us) + "}"
    );
}

void GhostRenderer::build(StGhostFrame const& frame, ScreenContext& screen) {
    mBuilt.clear();
    mBuildUs      = 0;
    mCellsBuilt   = 0;
    mSkippedCells = 0;
    float opacity = std::clamp(frame.opacity, 0.0f, 1.0f);
    auto  alpha   = static_cast<uint8_t>(std::lround(opacity * ALPHA_MAX));
    for (uint32_t i = 0; i < frame.job_count; ++i) {
        StGhostJob const& job = frame.jobs[i];
        // a job of another generation belongs to a build core has already replaced
        if (job.generation != frame.generation || job.slot >= mSlots.size() || mBuilt.size() >= mSlots.size()) continue;
        double start = FrameStatusReader::nowSeconds();
        bool   idle  = job.mode == ST_GHOST_JOB_MODE_REPLACE ? buildReplace(job, screen, alpha) : buildGroup(job, screen, alpha);
        // the game is tessellating: this job and the rest go unanswered, and core hands them out again
        if (!idle) break;
        mBuildUs += static_cast<uint64_t>((FrameStatusReader::nowSeconds() - start) * US_PER_SECOND);
    }
}

// tessellates the job into the slot's mesh; false when the game's tessellator is in use
bool GhostRenderer::buildReplace(StGhostJob const& job, ScreenContext& screen, uint8_t alpha) {
    GhostTessellator::Built built{};
    if (mUsable) built = mTessellator.build(screen.tessellator, job, mBlocks, alpha, mSlots[job.slot]);
    if (built.busy) return false;
    // the slot's mesh is no longer a group's
    mGroup.free(job.slot);
    mSizes[job.slot] = {job.slot_size[0], job.slot_size[1], job.slot_size[2]};
    mCellsBuilt += built.cells;
    mSkippedCells += built.skipped;
    mBuilt.push_back(StSlotBuilt{job.slot, job.serial, job.generation, built.vertices * GPU_BYTES_PER_VERTEX, built.ok});
    return true;
}

// an Append adds the job's vertices to the group buffer, a Finish uploads the buffer as the slot's mesh; a job the
// shim applied already is answered as built, any other job core sent out of turn is not answered. False when the
// game's tessellator is in use
bool GhostRenderer::buildGroup(StGhostJob const& job, ScreenContext& screen, uint8_t alpha) {
    GhostGroupBuffer::Step step = mGroup.step(job);
    if (step == GhostGroupBuffer::Step::Ignore) return true;
    if (step == GhostGroupBuffer::Step::Repeat || step == GhostGroupBuffer::Step::RepeatFinish) {
        uint64_t bytes = step == GhostGroupBuffer::Step::Repeat ? mGroup.vertices() * GPU_BYTES_PER_VERTEX
                                                                : mGroup.finishedBytes(job.slot);
        mBuilt.push_back(StSlotBuilt{job.slot, job.serial, job.generation, bytes, true});
        return true;
    }
    if (screen.tessellator.mTessellating) return false;
    if (job.group_index == 0) mGroup.start(job);
    if (job.mode == ST_GHOST_JOB_MODE_APPEND) {
        GhostTessellator::Built built{};
        if (mUsable) built = mTessellator.append(screen.tessellator, job, mBlocks, alpha, mGroup);
        if (built.busy) return false;
        // a failed part fails its group; core starts it again with index 0 or not at all
        if (!built.ok) {
            mGroup.drop();
            ModLog::info(
                "ghost group failed",
                "{\"slot\":" + std::to_string(job.slot) + ",\"group_serial\":" + std::to_string(job.group_serial)
                    + ",\"group_index\":" + std::to_string(job.group_index) + ",\"reason\":\""
                    + (mUsable ? "append refused" : "renderer not usable") + "\"}"
            );
        }
        mCellsBuilt += built.cells;
        mSkippedCells += built.skipped;
        mBuilt.push_back(StSlotBuilt{job.slot, job.serial, job.generation, mGroup.vertices() * GPU_BYTES_PER_VERTEX, built.ok});
        return true;
    }
    double start = FrameStatusReader::nowSeconds();
    GhostTessellator::Finished finished = mTessellator.finish(screen.tessellator, mGroup.data(), mSlots[job.slot]);
    if (finished.busy) return false;
    auto     us    = static_cast<uint64_t>((FrameStatusReader::nowSeconds() - start) * US_PER_SECOND);
    uint64_t bytes = finished.ok ? finished.vertices * GPU_BYTES_PER_VERTEX + finished.indices * INDEX_BYTES : 0;
    if (finished.ok) {
        mGroup.finish(job.slot, job.group_serial, bytes);
        mSizes[job.slot] = {job.slot_size[0], job.slot_size[1], job.slot_size[2]};
    } else {
        mGroup.drop();
        ModLog::info(
            "ghost group failed",
            "{\"slot\":" + std::to_string(job.slot) + ",\"group_serial\":" + std::to_string(job.group_serial)
                + ",\"group_index\":" + std::to_string(job.group_index) + ",\"reason\":\"finish refused\"}"
        );
    }
    mBuilt.push_back(StSlotBuilt{job.slot, job.serial, job.generation, bytes, finished.ok});
    ModLog::info(
        "ghost group uploaded",
        "{\"slot\":" + std::to_string(job.slot) + ",\"vertices\":" + std::to_string(finished.vertices)
            + ",\"indices\":" + std::to_string(finished.indices) + ",\"ok\":" + (finished.ok ? "true" : "false")
            + ",\"us\":" + std::to_string(us) + "}"
    );
    return true;
}

uint64_t GhostRenderer::draw(StGhostFrame const& frame, ScreenContext& screen, float const* camera) {
    mCulled      = 0;
    mMeshesDrawn = 0;
    if (!mBlend || !mAtlas) return 0;
    double   start = FrameStatusReader::nowSeconds();
    uint32_t count = cull(frame, screen, camera);
    for (uint32_t i = 0; i < count; ++i) {
        StGhostDraw const& draw = frame.draws[mOrder[i]];
        float origin[3] = {float(draw.origin[0]), float(draw.origin[1]), float(draw.origin[2])};
        drawMesh(screen, *mSlots[draw.slot], *mBlend, *mAtlas, origin, draw.offset, draw.scale, camera);
    }
    mMeshesDrawn = count;
    return static_cast<uint64_t>((FrameStatusReader::nowSeconds() - start) * US_PER_SECOND);
}

// fills mOrder with the indices of the listed draws that have a mesh and are in view, farthest first; a slot's box
// is outside when all eight corners lie beyond the same clip plane of projection x view, in camera-relative space.
// Returns how many are in mOrder and counts the rest in mCulled
uint32_t GhostRenderer::cull(StGhostFrame const& frame, ScreenContext& screen, float const* camera) {
    mce::Camera&      cam   = screen.camera;
    glm::mat4x4 const clip  = *cam.projectionMatrixStack->top()._m * *cam.viewMatrixStack->top()._m;
    uint32_t          count = 0;
    for (uint32_t i = 0; i < frame.draw_count && count < mOrder.size(); ++i) {
        StGhostDraw const& draw = frame.draws[i];
        if (draw.slot >= mSlots.size() || !mSlots[draw.slot]) continue;
        std::array<uint32_t, 3> const& size = mSizes[draw.slot];
        float lo[3] = {
            float(draw.origin[0]) - camera[0] + float(draw.offset[0]) * draw.scale,
            float(draw.origin[1]) - camera[1] + float(draw.offset[1]) * draw.scale,
            float(draw.origin[2]) - camera[2] + float(draw.offset[2]) * draw.scale,
        };
        float hi[3] = {
            lo[0] + float(size[0]) * draw.scale,
            lo[1] + float(size[1]) * draw.scale,
            lo[2] + float(size[2]) * draw.scale,
        };
        int   outside[PLANES] = {0, 0, 0, 0, 0, 0};
        for (int corner = 0; corner < CORNERS; ++corner) {
            glm::vec4 p = clip
                        * glm::vec4((corner & 1) ? hi[0] : lo[0], (corner & 2) ? hi[1] : lo[1], (corner & 4) ? hi[2] : lo[2], 1.0f);
            outside[0] += p.x < -p.w ? 1 : 0;
            outside[1] += p.x > p.w ? 1 : 0;
            outside[2] += p.y < -p.w ? 1 : 0;
            outside[3] += p.y > p.w ? 1 : 0;
            outside[4] += p.z < -p.w ? 1 : 0;
            outside[5] += p.z > p.w ? 1 : 0;
        }
        bool culled = false;
        for (int plane : outside) culled = culled || plane == CORNERS;
        if (culled) {
            ++mCulled;
            continue;
        }
        float dx               = (lo[0] + hi[0]) * HALF;
        float dy               = (lo[1] + hi[1]) * HALF;
        float dz               = (lo[2] + hi[2]) * HALF;
        mDistance[draw.slot]   = dx * dx + dy * dy + dz * dz;
        mOrder[count++]        = i;
    }
    // farthest first; equal distances fall back to the slot, so the order is the same every frame
    std::sort(mOrder.begin(), mOrder.begin() + count, [this, &frame](uint32_t a, uint32_t b) {
        uint32_t sa = frame.draws[a].slot;
        uint32_t sb = frame.draws[b].slot;
        float    da = mDistance[sa];
        float    db = mDistance[sb];
        return da != db ? da > db : sa < sb;
    });
    return count;
}

// fixed actor constants for a ghost: white tile light, white block light, no overlay, no change colours, no glint,
// UV animation offset 0 and scale 1. Without them a draw inherits the last actor's light and overlay
void GhostRenderer::setConstants(ScreenContext& screen) {
    mce::Color const none(0.0f, 0.0f, 0.0f, 0.0f);
    mce::Color const white(1.0f, 1.0f, 1.0f, 1.0f);
    Vec4             uvAnim{};
    uvAnim.z = 1.0f;
    uvAnim.w = 1.0f;
    ActorShaderManager::setEntityConstants(
        screen.constantBuffers.entityConstantBuffer.get(),
        screen.renderContext,
        white,
        Vec2(1.0f, 1.0f),
        glm::vec4(1.0f, 1.0f, 1.0f, 1.0f),
        none,
        none,
        none,
        none,
        Vec2(1.0f, 1.0f),
        uvAnim,
        0.0f,
        0.0f,
        0.0f,
        0.0f
    );
}

// each mesh gets its own world matrix: the world matrix times translate(origin - camera), scale(s) and
// translate(offset), composed with glm so the order does not depend on the game's Matrix methods. The mesh lands at
// origin + scale * (offset + position), relative to the camera
void GhostRenderer::drawMesh(
    ScreenContext&          screen,
    mce::Mesh const&        mesh,
    mce::MaterialPtr const& material,
    TextureVariant const&   texture,
    float const*            origin,
    int32_t const*          offset,
    float                   scale,
    float const*            camera
) {
    setConstants(screen);
    MatrixStack&                stack = screen.camera.worldMatrixStack.get();
    MatrixStack::MatrixStackRef ref   = stack.push(true);
    glm::mat4x4& world = *ref.mat->_m;
    world              = world * glm::translate(glm::mat4x4(1.0f), glm::vec3(origin[0] - camera[0], origin[1] - camera[1], origin[2] - camera[2]))
                * glm::scale(glm::mat4x4(1.0f), glm::vec3(scale, scale, scale))
                * glm::translate(glm::mat4x4(1.0f), glm::vec3(float(offset[0]), float(offset[1]), float(offset[2])));
    stack._isDirty = true;
    mesh.renderMesh(screen, material, texture, 0, 0, OffscreenCaptureDescription{}, nullptr);
}

} // namespace stratum
