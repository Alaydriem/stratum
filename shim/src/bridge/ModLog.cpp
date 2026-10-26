#include "bridge/ModLog.h"

#include "alaydriem_mod_manager.h"
#include "stratum.h"

#include <cstdio>
#include <cstring>

namespace stratum {

namespace {

constexpr unsigned char FIRST_PRINTABLE = 0x20;
constexpr size_t        ESCAPE_LEN      = 7;

uint8_t const* bytes(std::string_view text) { return reinterpret_cast<uint8_t const*>(text.data()); }

} // namespace

void ModLog::error(std::string_view msg, std::string_view fields, std::source_location at) {
    write(AMM_LOG_LEVEL_ERROR, msg, fields, at);
}

void ModLog::warn(std::string_view msg, std::string_view fields, std::source_location at) {
    write(AMM_LOG_LEVEL_WARN, msg, fields, at);
}

void ModLog::info(std::string_view msg, std::string_view fields, std::source_location at) {
    write(AMM_LOG_LEVEL_INFO, msg, fields, at);
}

void ModLog::debug(std::string_view msg, std::string_view fields, std::source_location at) {
    write(AMM_LOG_LEVEL_DEBUG, msg, fields, at);
}

std::string ModLog::quote(std::string_view text) {
    std::string out;
    out.reserve(text.size() + 2);
    out.push_back('"');
    for (char c : text) {
        auto u = static_cast<unsigned char>(c);
        if (c == '"' || c == '\\') {
            out.push_back('\\');
            out.push_back(c);
        } else if (u < FIRST_PRINTABLE) {
            char escaped[ESCAPE_LEN + 1]{};
            std::snprintf(escaped, sizeof(escaped), "\\u%04x", static_cast<unsigned>(u));
            out.append(escaped, std::strlen(escaped));
        } else {
            out.push_back(c);
        }
    }
    out.push_back('"');
    return out;
}

void ModLog::write(uint8_t level, std::string_view msg, std::string_view fields, std::source_location const& at) {
    std::string_view file = at.file_name();
    st_log(level, bytes(msg), msg.size(), bytes(fields), fields.size(), bytes(file), file.size(), at.line());
}

} // namespace stratum
