#pragma once

#include <cstdint>
#include <source_location>
#include <string>
#include <string_view>

namespace stratum {

// shim logging: every line goes up through the mod's manager registration to the mod manager's log, with the
// file and line of the caller. `fields` is a JSON object with the varying data; the message itself stays fixed
// text. Lines written while the mod is not registered are dropped
class ModLog {
public:
    static void error(std::string_view msg, std::string_view fields = "{}", std::source_location at = std::source_location::current());

    static void warn(std::string_view msg, std::string_view fields = "{}", std::source_location at = std::source_location::current());

    static void info(std::string_view msg, std::string_view fields = "{}", std::source_location at = std::source_location::current());

    static void debug(std::string_view msg, std::string_view fields = "{}", std::source_location at = std::source_location::current());

    // `text` as a JSON string literal, quotes included, for a text field value
    static std::string quote(std::string_view text);

private:
    static void write(uint8_t level, std::string_view msg, std::string_view fields, std::source_location const& at);
};

} // namespace stratum
