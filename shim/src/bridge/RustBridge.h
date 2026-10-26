#pragma once

#include "stratum.h"

#include <filesystem>
#include <string>

namespace stratum {

// owns the Rust handle; every call into Rust goes through handle()
class RustBridge {
public:
    // registers with the mod manager; false when the manager is not loaded or refuses the mod
    static bool create(std::filesystem::path const& configDir, std::filesystem::path const& dataDir);

    static void destroy();

    // null before create and after destroy
    static StHandle* handle();

    static std::string utf8(std::filesystem::path const& path);

    // a borrowed view of `text`, valid while `text` is alive and unchanged
    static StStr view(std::string const& text);
};

} // namespace stratum
