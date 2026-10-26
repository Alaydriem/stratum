#include "ghost/GhostGroupBuffer.h"

#include <glm/glm.hpp>

namespace stratum {

namespace {

constexpr int UV_SETS = 3;

} // namespace

void GhostGroupBuffer::resize(size_t slots) {
    mData.reset();
    mFinished.assign(slots, std::nullopt);
    mFinishedBytes.assign(slots, 0);
}

GhostGroupBuffer::Step GhostGroupBuffer::step(StGhostJob const& job) const {
    if (mData && job.slot == mSlot && job.group_serial == mSerial) {
        if (job.group_index == mNext) return Step::Apply;
        if (job.group_index < mNext) return Step::Repeat;
        return Step::Ignore;
    }
    if (job.mode == ST_GHOST_JOB_MODE_FINISH && job.slot < mFinished.size() && mFinished[job.slot] == job.group_serial) {
        return Step::RepeatFinish;
    }
    return job.group_index == 0 ? Step::Apply : Step::Ignore;
}

void GhostGroupBuffer::start(StGhostJob const& job) {
    mData.reset();
    mData.emplace();
    mData->mMode = mce::PrimitiveMode::TriangleList;
    mSlot        = job.slot;
    mSerial      = job.group_serial;
    mNext        = 0;
}

bool GhostGroupBuffer::add(mce::MeshData const& data) {
    if (!mData) return false;
    mce::MeshData& to    = *mData;
    size_t         had   = to.mPositions->size();
    size_t         count = data.mPositions->size();
    size_t         total = had + count;
    // a field filled by some jobs and not by others would give later vertices another vertex's value
    bool fits = data.mIndices->empty() && covers(*to.mNormals, *data.mNormals, total)
             && covers(*to.mTangents, *data.mTangents, total) && covers(*to.mColors, *data.mColors, total)
             && covers(*to.mBoneId0s, *data.mBoneId0s, total)
             && covers(*to.mPBRTextureIndices, *data.mPBRTextureIndices, total)
             && covers(*to.mMERS, *data.mMERS, total) && covers(*to.mGeoType, *data.mGeoType, total);
    for (int uv = 0; uv < UV_SETS && fits; ++uv) {
        fits = covers(to.mTextureUVs[uv].get(), data.mTextureUVs[uv].get(), total);
    }
    if (!fits) return false;
    if (count > 0) {
        bool unfilled = had == 0;
        auto&       box   = to.mAABB.get();
        auto const& add   = data.mAABB.get();
        auto&       uvBox = to.mUVAABB.get();
        auto const& uvAdd = data.mUVAABB.get();
        box.first   = unfilled ? add.first : glm::min(box.first, add.first);
        box.second  = unfilled ? add.second : glm::max(box.second, add.second);
        uvBox.first = unfilled ? uvAdd.first : glm::min(uvBox.first, uvAdd.first);
        uvBox.second = unfilled ? uvAdd.second : glm::max(uvBox.second, uvAdd.second);
    }
    extend(to.mPositions.get(), data.mPositions.get());
    extend(to.mNormals.get(), data.mNormals.get());
    extend(to.mTangents.get(), data.mTangents.get());
    extend(to.mColors.get(), data.mColors.get());
    extend(to.mBoneId0s.get(), data.mBoneId0s.get());
    for (int uv = 0; uv < UV_SETS; ++uv) extend(to.mTextureUVs[uv].get(), data.mTextureUVs[uv].get());
    extend(to.mPBRTextureIndices.get(), data.mPBRTextureIndices.get());
    extend(to.mMERS.get(), data.mMERS.get());
    extend(to.mGeoType.get(), data.mGeoType.get());
    auto&       enabled = to.mFieldEnabled.get();
    auto const& more    = data.mFieldEnabled.get();
    for (size_t i = 0; i < enabled.size(); ++i) enabled[i] = enabled[i] || more[i];
    ++mNext;
    return true;
}

mce::MeshData& GhostGroupBuffer::data() { return *mData; }

uint64_t GhostGroupBuffer::vertices() const { return mData ? mData->mPositions->size() : 0; }

void GhostGroupBuffer::finish(uint32_t slot, uint32_t serial, uint64_t bytes) {
    mData.reset();
    if (slot >= mFinished.size()) return;
    mFinished[slot]      = serial;
    mFinishedBytes[slot] = bytes;
}

uint64_t GhostGroupBuffer::finishedBytes(uint32_t slot) const {
    return slot < mFinishedBytes.size() ? mFinishedBytes[slot] : 0;
}

void GhostGroupBuffer::drop() { mData.reset(); }

void GhostGroupBuffer::free(uint32_t slot) {
    if (mData && mSlot == slot) mData.reset();
    if (slot >= mFinished.size()) return;
    mFinished[slot]      = std::nullopt;
    mFinishedBytes[slot] = 0;
}

void GhostGroupBuffer::clear() {
    mData.reset();
    mFinished.assign(mFinished.size(), std::nullopt);
    mFinishedBytes.assign(mFinishedBytes.size(), 0);
}

} // namespace stratum
