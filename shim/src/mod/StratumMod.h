#pragma once

#include "ll/api/mod/NativeMod.h"

#include <filesystem>

namespace stratum {

// the LeviLamina entry point: registers with the mod manager through Rust and installs the hooks
class StratumMod {
public:
    static StratumMod& getInstance();

    StratumMod() : mSelf(*ll::mod::NativeMod::current()) {}

    [[nodiscard]] ll::mod::NativeMod& getSelf() const { return mSelf; }

    bool load();

    bool enable();

    bool disable();

private:
    ll::mod::NativeMod&   mSelf;
    std::filesystem::path mConfigDir;
    std::filesystem::path mDataDir;
};

} // namespace stratum
