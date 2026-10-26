#pragma once

#include <windows.h>

#include <cstdint>
#include <string>
#include <vector>

namespace stratum {

// one answer of the structure file picker. `status` is the first failing COM call's HRESULT, or S_OK;
// a cancel is HRESULT_FROM_WIN32(ERROR_CANCELLED). `dropped` counts chosen items whose path could not be
// read, `invalid` the paths that were not valid UTF-16
struct PickResult {
    HRESULT                  status = S_OK;
    std::vector<std::string> paths;
    uint32_t                 dropped = 0;
    uint32_t                 invalid = 0;
};

} // namespace stratum
