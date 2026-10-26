#include "state/WorldKeyReader.h"

#include "bridge/ModLog.h"

#include <ll/api/service/Bedrock.h>
#include <mc/client/game/ClientInstance.h>
#include <mc/client/multiplayer/ClientLevel.h>
#include <mc/network/ConnectionType.h>
#include <mc/network/GameConnectionInfo.h>
#include <mc/server/ServerInstance.h>
#include <mc/world/level/LevelSeed64.h>
#include <mc/world/level/storage/LevelData.h>

#include <optional>
#include <string_view>

namespace stratum {

namespace {

// client updates in one world before a world with neither a local nor a server source takes the name key
constexpr int FALLBACK_AFTER_READS = 300;
// client updates in a row with the same server key and isInRealm() false before that key is taken
constexpr int SERVER_STABLE_READS = 20;
// client updates in one world during which a server: or name: key is given up when isInRealm() turns true
constexpr int REALM_RECHECK_READS = 600;

constexpr std::string_view LOCAL_PREFIX  = "local:";
constexpr std::string_view SERVER_PREFIX = "server:";
constexpr std::string_view REALM_PREFIX  = "realm:";
constexpr std::string_view NAME_PREFIX   = "name:";

enum class KeyKind { Undecided, Local, Server, Realm, Name };

// the client level of the last read; compared by address only, never dereferenced
void const* g_level    = nullptr;
KeyKind     g_kind     = KeyKind::Undecided;
int         g_attempts = 0;
// the key read last while it is not yet taken, and how many reads in a row gave that server key; one key
std::string g_candidate;
int         g_candidateReads = 0;

} // namespace

void WorldKeyReader::read(ClientInstance& client, std::string& key) {
    ClientLevel const* level = client.getLevel();
    if (!level) {
        reset();
        key.clear();
        return;
    }
    if (g_level != level) {
        reset();
        g_level = level;
    }
    // local: and realm: are final; server: and name: give way to a Realm during the first updates
    if (g_kind == KeyKind::Local || g_kind == KeyKind::Realm) return;
    if (g_kind != KeyKind::Undecided && g_attempts >= REALM_RECHECK_READS) return;
    ++g_attempts;
    bool inRealm = client.isInRealm();
    if (inRealm) {
        g_candidateReads = 0;
        g_candidate.clear();
        if (readRealm(client, g_candidate)) {
            key    = g_candidate;
            g_kind = KeyKind::Realm;
            logChosen(key, inRealm, g_attempts);
            return;
        }
        // a Realm whose level name is not known yet never keeps a server key
        if (g_kind == KeyKind::Server) {
            g_kind = KeyKind::Undecided;
            key.clear();
        }
        if (g_kind != KeyKind::Undecided) return;
        key.clear();
        if (g_attempts < FALLBACK_AFTER_READS) return;
        readName(client, key);
        g_kind = KeyKind::Name;
        logChosen(key, inRealm, g_attempts);
        return;
    }
    if (g_kind != KeyKind::Undecided) return;
    key.clear();
    std::optional<Social::GameConnectionInfo> connection = client.getGameConnectionInfo();
    Social::GameConnectionInfo const*         info       = connection ? &*connection : nullptr;
    if (readLocal(info, key)) {
        g_kind = KeyKind::Local;
        logChosen(key, inRealm, g_attempts);
        return;
    }
    if (readServer(info, key)) {
        if (key == g_candidate) {
            ++g_candidateReads;
        } else {
            g_candidate      = key;
            g_candidateReads = 1;
        }
        if (g_candidateReads < SERVER_STABLE_READS && g_attempts < FALLBACK_AFTER_READS) {
            key.clear();
            return;
        }
        g_kind = KeyKind::Server;
        logChosen(key, inRealm, g_attempts);
        return;
    }
    g_candidateReads = 0;
    g_candidate.clear();
    if (g_attempts < FALLBACK_AFTER_READS) return;
    readName(client, key);
    g_kind = KeyKind::Name;
    logChosen(key, inRealm, g_attempts);
}

void WorldKeyReader::reset() {
    g_level          = nullptr;
    g_kind           = KeyKind::Undecided;
    g_attempts       = 0;
    g_candidateReads = 0;
    g_candidate.clear();
}

bool WorldKeyReader::readLocal(Social::GameConnectionInfo const* connection, std::string& key) {
    if (connection) {
        auto type = static_cast<Social::ConnectionType>(connection->mType);
        if (type != Social::ConnectionType::Local && !isLoopback(connection->mHostIpAddress)
            && !isLoopback(connection->mUnresolvedUrl)) {
            return false;
        }
    }
    auto server = ll::service::getServerInstance();
    if (!server) return false;
    if (server->mInstanceState->load(std::memory_order_seq_cst) != ServerInstance::InstanceState::Running) {
        return false;
    }
    std::string levelId = server->getLevelId();
    if (levelId.empty()) return false;
    key.append(LOCAL_PREFIX).append(levelId);
    return true;
}

bool WorldKeyReader::readServer(Social::GameConnectionInfo const* connection, std::string& key) {
    if (!connection) return false;
    auto type = static_cast<Social::ConnectionType>(connection->mType);
    if (type == Social::ConnectionType::Local || type == Social::ConnectionType::Undefined) return false;
    std::string const& url     = connection->mUnresolvedUrl;
    std::string const& address = url.empty() ? static_cast<std::string const&>(connection->mHostIpAddress) : url;
    int                port    = connection->mPort;
    if (address.empty() || port <= 0) return false;
    key.append(SERVER_PREFIX);
    bool bracket = address.find(':') != std::string::npos && address.front() != '[';
    if (bracket) key.append("[");
    key.append(address);
    if (bracket) key.append("]");
    key.append(":").append(std::to_string(port));
    return true;
}

bool WorldKeyReader::readRealm(ClientInstance& client, std::string& key) {
    ClientLevel* level = client.getLevel();
    if (!level) return false;
    std::string const& name = level->getLevelData().mLevelName;
    if (name.empty()) return false;
    key.append(REALM_PREFIX).append(name);
    return true;
}

void WorldKeyReader::readName(ClientInstance& client, std::string& key) {
    ClientLevel* level = client.getLevel();
    if (!level) return;
    std::string const& name = level->getLevelData().mLevelName;
    uint64             seed = level->getLevelSeed64().mValue;
    key.append(NAME_PREFIX).append(name).append(":").append(std::to_string(seed));
}

bool WorldKeyReader::isLoopback(std::string const& address) {
    return address == "localhost" || address == "::1" || address == "[::1]" || address.starts_with("127.");
}

void WorldKeyReader::logChosen(std::string const& key, bool inRealm, int attempts) {
    std::string_view kind = std::string_view(key).substr(0, key.find(':'));
    ModLog::info(
        "world key",
        "{\"kind\":" + ModLog::quote(kind) + ",\"key\":" + ModLog::quote(key) + ",\"in_realm\":"
            + std::string(inRealm ? "true" : "false") + ",\"reads\":" + std::to_string(attempts) + "}"
    );
}

} // namespace stratum
