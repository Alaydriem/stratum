#include "mod/StratumMod.h"

#include "bridge/ModLog.h"
#include "bridge/RustBridge.h"
#include "easyplace/UseHooks.h"
#include "ghost/GhostRenderer.h"
#include "hooks/ClientUpdateHook.h"
#include "hooks/GameFocusLostHook.h"
#include "picker/PickerPump.h"
#include "state/WorldKeyReader.h"

#include "ll/api/mod/RegisterHelper.h"

#include <system_error>

namespace stratum {

namespace {

// the mod manager and the library require absolute paths
std::filesystem::path absoluteDir(std::filesystem::path const& dir) {
    std::error_code       error;
    std::filesystem::path absolute = std::filesystem::absolute(dir, error);
    return error ? dir : absolute;
}

} // namespace

StratumMod& StratumMod::getInstance() {
    static StratumMod instance;
    return instance;
}

// load runs under the OS loader lock, so registration waits for enable
bool StratumMod::load() {
    mConfigDir = absoluteDir(getSelf().getConfigDir());
    mDataDir   = absoluteDir(getSelf().getDataDir());
    return true;
}

bool StratumMod::enable() {
    // a disable destroys the core, so an enable after it registers again
    if (!RustBridge::handle() && !RustBridge::create(mConfigDir, mDataDir)) {
        // the manager's log is not reachable without a registration, so this one line goes to the LeviLamina console
        getSelf().getLogger().error("alaydriem-mod-manager is not loaded or refused stratum; the mod stays off");
        return true;
    }
    ClientUpdateHook::hook();
    GameFocusLostHook::hook();
    GhostRenderer::start();
    UseScreenGoneHook::hook();
    UseHudTickHook::hook();
    UseStackResponseHook::hook();
    UseContainerOpenHook::hook();
    UseContainerCloseHook::hook();
    UseBuildBlockHook::hook();
    UseBuildActionHook::hook();
    ModLog::info(
        "stratum enabled",
        "{\"config_dir\":" + ModLog::quote(RustBridge::utf8(mConfigDir))
            + ",\"data_dir\":" + ModLog::quote(RustBridge::utf8(mDataDir)) + "}"
    );
    return true;
}

bool StratumMod::disable() {
    if (!RustBridge::handle()) return true;
    // unhook first, in reverse install order, so no game call can enter Rust, then end the picker, then unregister and
    // stop Rust
    UseBuildActionHook::unhook();
    UseBuildBlockHook::unhook();
    UseContainerCloseHook::unhook();
    UseContainerOpenHook::unhook();
    UseStackResponseHook::unhook();
    UseHudTickHook::unhook();
    UseScreenGoneHook::unhook();
    UseDriver::reset();
    GhostRenderer::stop();
    GameFocusLostHook::unhook();
    ClientUpdateHook::unhook();
    PickerPump::stop();
    WorldKeyReader::reset();
    RustBridge::destroy();
    return true;
}

} // namespace stratum

LL_REGISTER_MOD(stratum::StratumMod, stratum::StratumMod::getInstance());
