#include "bridge/RustBridge.h"

#include <atomic>

namespace stratum {

namespace {

std::atomic<StHandle*> g_handle{nullptr};

} // namespace

bool RustBridge::create(std::filesystem::path const& configDir, std::filesystem::path const& dataDir) {
    std::string config = utf8(configDir);
    std::string data   = utf8(dataDir);
    StStr       configText = view(config);
    StStr       dataText   = view(data);
    StHandle*   handle     = st_create(&configText, &dataText);
    g_handle.store(handle, std::memory_order_release);
    return handle != nullptr;
}

void RustBridge::destroy() {
    StHandle* handle = g_handle.exchange(nullptr, std::memory_order_acq_rel);
    if (handle) st_destroy(handle);
}

StHandle* RustBridge::handle() { return g_handle.load(std::memory_order_acquire); }

std::string RustBridge::utf8(std::filesystem::path const& path) {
    auto bytes = path.u8string();
    return std::string(reinterpret_cast<char const*>(bytes.data()), bytes.size());
}

StStr RustBridge::view(std::string const& text) {
    return StStr{reinterpret_cast<uint8_t const*>(text.data()), text.size()};
}

} // namespace stratum
