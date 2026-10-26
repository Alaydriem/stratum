#pragma once

#include "picker/PickResult.h"

#include <optional>
#include <string>

namespace stratum {

// the Windows file-open dialog, filtered to .mcstructure, with more than one file allowed. Blocks; call it from
// a worker thread
class StructureFilePicker {
public:
    // the chosen paths as UTF-8, with the dialog's status; no paths when it was cancelled or could not open
    static PickResult pick();

private:
    // empty when `wide` is not valid UTF-16
    static std::optional<std::string> toUtf8(wchar_t const* wide);
};

} // namespace stratum
