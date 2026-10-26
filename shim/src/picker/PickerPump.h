#pragma once

#include "stratum.h"

namespace stratum {

// runs the structure file picker on one worker thread at a time and hands its answer to Rust on the game
// thread. Rust decides when to pick and what to do with the files; this only moves the request and the answer
class PickerPump {
public:
    // on the game thread: opens the picker on a worker thread; ignored while one is open or not yet delivered
    static void request();

    // on the game thread, once per tick: sends a finished pick to Rust through st_files_picked
    static void deliver(StHandle* handle);

    // on disable: closes an open dialog as a cancel, joins the worker and drops an undelivered answer. The
    // close is retried for a bounded time and logged when it runs out; the join after it is unbounded, because
    // a detached worker could outlive this module's code and data
    static void stop();
};

} // namespace stratum
