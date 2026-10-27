#include "easyplace/ProbeReader.h"

#include <mc/client/player/LocalPlayer.h>
#include <mc/common/FacingID.h>
#include <mc/deps/core/math/Vec3.h>
#include <mc/world/gamemode/GameMode.h>
#include <mc/world/level/BlockPos.h>
#include <mc/world/level/BlockSource.h>
#include <mc/world/level/GameType.h>
#include <mc/world/level/Level.h>
#include <mc/world/level/block/Block.h>
#include <mc/world/level/block/BlockType.h>
#include <mc/world/phys/HitResult.h>
#include <mc/world/phys/HitResultType.h>

#include <cmath>
#include <limits>

namespace stratum {

namespace {

constexpr uint8_t GAME_MODE_SURVIVAL = 0;
constexpr uint8_t GAME_MODE_CREATIVE = 1;
constexpr uint8_t GAME_MODE_OTHER    = 2;
constexpr uint8_t FACE_COUNT         = 6;
constexpr double  MIN_LENGTH         = 1e-9;

// the face a ray enters a cell through, by the axis it stepped on and whether the step was positive: a step to +X
// enters through the cell's West face
constexpr uint8_t ENTERED_FACE[3][2] = {
    {static_cast<uint8_t>(FacingID::West), static_cast<uint8_t>(FacingID::East)},
    {static_cast<uint8_t>(FacingID::Down), static_cast<uint8_t>(FacingID::Up)},
    {static_cast<uint8_t>(FacingID::North), static_cast<uint8_t>(FacingID::South)},
};

bool replaceable(BlockSource& region, BlockPos const& pos) {
    Block const& block = region.getBlock(pos);
    return block.getBlockType().canBeBuiltOver(block, region, pos);
}

} // namespace

void ProbeReader::read(LocalPlayer& player, BlockSource& region, GameMode& gameMode, StUseProbe& probe) {
    Vec3 eye = player.getEyePos();
    Vec3 dir = player.getViewVector(1.0f);
    probe.eye[0] = eye.x;
    probe.eye[1] = eye.y;
    probe.eye[2] = eye.z;
    double length = std::sqrt(double(dir.x) * dir.x + double(dir.y) * dir.y + double(dir.z) * dir.z);
    if (length > MIN_LENGTH) {
        probe.dir[0] = dir.x / length;
        probe.dir[1] = dir.y / length;
        probe.dir[2] = dir.z / length;
    }
    probe.reach = gameMode.getMaxPickRange();
    switch (player.getPlayerGameType()) {
    case GameType::Survival:
        probe.game_mode = GAME_MODE_SURVIVAL;
        break;
    case GameType::Creative:
        probe.game_mode = GAME_MODE_CREATIVE;
        break;
    default:
        probe.game_mode = GAME_MODE_OTHER;
        break;
    }
    probe.has_hit = false;
    if (length > MIN_LENGTH && walk(region, probe)) return;
    gameHit(player, region, probe);
}

// steps the ray from the eye cell to each cell it enters, nearest first, and stops at the first cell past the reach
bool ProbeReader::walk(BlockSource& region, StUseProbe& probe) {
    constexpr double infinity = std::numeric_limits<double>::infinity();
    int    cell[3];
    int    step[3];
    double next[3];
    double delta[3];
    for (int axis = 0; axis < 3; ++axis) {
        double origin = probe.eye[axis];
        double d      = probe.dir[axis];
        cell[axis]    = static_cast<int>(std::floor(origin));
        if (d == 0.0) {
            step[axis]  = 0;
            delta[axis] = infinity;
            next[axis]  = infinity;
            continue;
        }
        step[axis]  = d > 0.0 ? 1 : -1;
        delta[axis] = std::abs(1.0 / d);
        next[axis]  = (d > 0.0 ? (cell[axis] + 1 - origin) : (origin - cell[axis])) * delta[axis];
    }
    for (int i = 0; i < MAX_CROSSINGS; ++i) {
        int axis = next[0] < next[1] ? (next[0] < next[2] ? 0 : 2) : (next[1] < next[2] ? 1 : 2);
        double t = next[axis];
        if (t > probe.reach) return false;
        cell[axis] += step[axis];
        next[axis] += delta[axis];
        BlockPos pos(cell[0], cell[1], cell[2]);
        if (replaceable(region, pos)) continue;
        probe.has_hit         = true;
        probe.hit_cell[0]     = cell[0];
        probe.hit_cell[1]     = cell[1];
        probe.hit_cell[2]     = cell[2];
        probe.hit_face        = ENTERED_FACE[axis][step[axis] > 0 ? 0 : 1];
        probe.hit_distance    = t;
        probe.hit_replaceable = false;
        return true;
    }
    return false;
}

void ProbeReader::gameHit(LocalPlayer& player, BlockSource& region, StUseProbe& probe) {
    HitResult const& hit = player.getLevel().getHitResult();
    if (hit.mType != HitResultType::Tile || hit.mFacing >= FACE_COUNT) return;
    double dx             = hit.mPos.x - probe.eye[0];
    double dy             = hit.mPos.y - probe.eye[1];
    double dz             = hit.mPos.z - probe.eye[2];
    probe.has_hit         = true;
    probe.hit_cell[0]     = hit.mBlock.x;
    probe.hit_cell[1]     = hit.mBlock.y;
    probe.hit_cell[2]     = hit.mBlock.z;
    probe.hit_face        = hit.mFacing;
    probe.hit_distance    = std::sqrt(dx * dx + dy * dy + dz * dz);
    probe.hit_replaceable = replaceable(region, hit.mBlock);
}

} // namespace stratum
