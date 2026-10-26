#pragma once

#include <string>

class ClientInstance;

namespace Social {
class GameConnectionInfo;
}

namespace stratum {

// names the loaded world with a key that stays the same for one world across game restarts and differs
// between worlds: `local:<storage folder id>`, `server:<address>:<port>`, `realm:<level name>`
// or `name:<level name>:<seed>`.
// It reads the game again while the client level is new and no kind was taken, and, for a server: or name:
// key, during the first client updates in case isInRealm() turns true late. A Realm key needs a non-empty
// level name, and a server: key is taken only after the same key with isInRealm() false on 20 updates in a
// row. Each chosen key is logged at info level.
//
// The local server instance comes from ll::service::getServerInstance(). The reader relies on LeviLamina
// clearing that service when the game destroys the instance; it dereferences the instance only on the game
// thread, in the call that fetched it, and only when its state is Running
class WorldKeyReader {
public:
    // writes the key of the world `client` shows into `key`; empty outside a world and while the kind is
    // still being decided
    static void read(ClientInstance& client, std::string& key);

    // forgets the last world, so the next read in a world reads the game again
    static void reset();

private:
    static bool readLocal(Social::GameConnectionInfo const* connection, std::string& key);

    static bool readServer(Social::GameConnectionInfo const* connection, std::string& key);

    // false while the level name is empty
    static bool readRealm(ClientInstance& client, std::string& key);

    static void readName(ClientInstance& client, std::string& key);

    static bool isLoopback(std::string const& address);

    static void logChosen(std::string const& key, bool inRealm, int attempts);
};

} // namespace stratum
