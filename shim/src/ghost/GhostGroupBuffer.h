#pragma once

#include "stratum.h"

#include <mc/deps/minecraft_renderer/renderer/MeshData.h>

#include <cstddef>
#include <cstdint>
#include <optional>
#include <vector>

namespace stratum {

// the one mini ghost group the shim is building: the vertices of the Append jobs applied so far, in one CPU
// MeshData, and the job identity they belong to (slot, group build serial, next index); and per slot the group build
// serial and bytes of the group it finished last. Bounded: one buffer at a time, its size held by core's group cell
// limit, and the per-slot records sized with the slot table
class GhostGroupBuffer {
public:
    // what a group job asks for, against what the shim holds
    enum class Step {
        // the next job of the buffer's group build, or index 0 of a new one
        Apply,
        // a job of the buffer's group build that was applied already: answered with the buffer's bytes
        Repeat,
        // the Finish of the slot's last finished group: answered with that group's bytes
        RepeatFinish,
        // not answered; core hands it out again
        Ignore,
    };

    // sized with the slot table; holds nothing after
    void resize(size_t slots);

    [[nodiscard]] Step step(StGhostJob const& job) const;

    // an empty buffer for the job's slot and group build, in place of any other; the job is at index 0
    void start(StGhostJob const& job);

    // adds every vertex field of `data` to the buffer and moves on to the next index. False, with the buffer as it
    // was before the call, when `data` has indices of its own or a field that does not cover every vertex
    bool add(mce::MeshData const& data);

    // the buffer's vertices for an upload; the buffer is dropped by `finish`
    [[nodiscard]] mce::MeshData& data();

    [[nodiscard]] uint64_t vertices() const;

    // drops the buffer and records the slot's finished group with what its mesh holds
    void finish(uint32_t slot, uint32_t serial, uint64_t bytes);

    [[nodiscard]] uint64_t finishedBytes(uint32_t slot) const;

    // drops the buffer
    void drop();

    // drops the buffer if it is for `slot`, and the slot's finished record
    void free(uint32_t slot);

    // drops the buffer and every finished record
    void clear();

private:
    std::optional<mce::MeshData> mData;
    uint32_t                     mSlot   = 0;
    uint32_t                     mSerial = 0;
    uint32_t                     mNext   = 0;
    // per slot: the group build serial and bytes of the group it finished last; empty when it has none
    std::vector<std::optional<uint32_t>> mFinished;
    std::vector<uint64_t>                mFinishedBytes;

    template <typename T>
    static void extend(std::vector<T>& to, std::vector<T> const& from) {
        to.insert(to.end(), from.begin(), from.end());
    }

    // a field the buffer and the new vertices together hold for none or for all of `vertices`
    template <typename T>
    static bool covers(std::vector<T> const& had, std::vector<T> const& more, size_t vertices) {
        size_t held = had.size() + more.size();
        return held == 0 || held == vertices;
    }
};

} // namespace stratum
