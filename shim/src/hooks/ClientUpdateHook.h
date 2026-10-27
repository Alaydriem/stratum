#pragma once

#include "bridge/ModLog.h"
#include "bridge/RustBridge.h"
#include "easyplace/UseDriver.h"
#include "picker/PickerPump.h"
#include "state/FrameStatusReader.h"

#include <ll/api/memory/Hook.h>
#include <mc/client/game/ClientInstance.h>

#include <string>

namespace stratum {

// once per client tick, after the game's own update: hands a finished pick to Rust, runs the tick, opens the
// picker when Rust asked for it, then runs the easy place hold
LL_TYPE_INSTANCE_HOOK(
    ClientUpdateHook,
    ll::memory::HookPriority::Normal,
    ClientInstance,
    &ClientInstance::$update,
    bool,
    bool a1
) {
    bool      result = origin(a1);
    StHandle* handle = RustBridge::handle();
    if (!handle) return result;
    // the world key of the loaded world; one string, rewritten when the world changes
    static std::string worldStorage;
    StFrameStatus      status{};
    FrameStatusReader::read(*this, status, worldStorage);
    PickerPump::deliver(handle);
    // the last tick's status, so a failure and a recovery are each logged once
    static StStatus lastTick = ST_STATUS_OK;
    StStatus        tick     = st_tick(handle, &status);
    if (tick != lastTick) {
        std::string fields = "{\"status\":" + std::to_string(static_cast<int>(tick)) + "}";
        if (tick == ST_STATUS_OK) {
            ModLog::info("st_tick recovered", fields);
        } else {
            ModLog::warn("st_tick failed", fields);
        }
        lastTick = tick;
    }
    if (st_take_picker_request(handle)) PickerPump::request();
    UseDriver::tick(*this);
    return result;
}

} // namespace stratum
