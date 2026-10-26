#include "state/FrameStatusReader.h"

#include "bridge/RustBridge.h"
#include "state/WorldKeyReader.h"

#include <mc/client/game/ClientInstance.h>
#include <mc/client/options/IOptionRegistry.h>
#include <mc/client/player/LocalPlayer.h>
#include <mc/world/level/BlockPos.h>
#include <mc/world/level/dimension/DimensionType.h>

#include <windows.h>

namespace stratum {

double FrameStatusReader::nowSeconds() {
    static double const frequency = [] {
        LARGE_INTEGER value;
        QueryPerformanceFrequency(&value);
        return static_cast<double>(value.QuadPart);
    }();
    LARGE_INTEGER now;
    QueryPerformanceCounter(&now);
    return static_cast<double>(now.QuadPart) / frequency;
}

void FrameStatusReader::read(ClientInstance& client, StFrameStatus& status, std::string& worldStorage) {
    status              = StFrameStatus{};
    status.time_seconds = nowSeconds();
    if (!client.isWorldActive()) {
        WorldKeyReader::reset();
        worldStorage.clear();
        status.world = RustBridge::view(worldStorage);
        return;
    }
    status.in_world = true;
    WorldKeyReader::read(client, worldStorage);
    LocalPlayer* player = client.getLocalPlayer();
    // with no player the dimension is not known, so the world key stays empty and Rust loads no marker set yet
    if (!player) {
        status.world = StStr{};
        return;
    }
    status.world      = RustBridge::view(worldStorage);
    BlockPos feet     = player->getFeetBlockPos();
    status.has_player = true;
    status.menu_open  = client.isShowingMenu();
    status.dimension  = static_cast<int32_t>(player->getDimensionId());
    status.feet_x     = feet.x;
    status.feet_y     = feet.y;
    status.feet_z     = feet.z;
    // the video setting, in chunks; the ghost draws this far unless the config names its own distance
    int chunks             = client.getOptions().getViewDistanceChunks();
    status.render_distance = chunks > 0 ? static_cast<uint32_t>(chunks) : 0;
}

} // namespace stratum
