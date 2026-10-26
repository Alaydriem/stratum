#include "picker/PickerPump.h"

#include "bridge/ModLog.h"
#include "bridge/RustBridge.h"
#include "picker/StructureFilePicker.h"

#include <windows.h>

#include <atomic>
#include <cstdio>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <vector>

namespace stratum {

namespace {

// how long stop waits for the dialog to close before it joins anyway
constexpr int   CLOSE_ATTEMPTS = 100;
constexpr DWORD CLOSE_WAIT_MS  = 50;

std::jthread                            g_worker;
std::atomic<bool>                       g_busy{false};
std::atomic<bool>                       g_done{false};
std::atomic<DWORD>                      g_workerThreadId{0};
std::mutex                              g_mutex;
std::optional<PickResult>               g_answer;

std::string hex(HRESULT value) {
    char text[16]{};
    std::snprintf(text, sizeof(text), "0x%08lX", static_cast<unsigned long>(value));
    return ModLog::quote(text);
}

BOOL CALLBACK closeWindow(HWND window, LPARAM) {
    PostMessageW(window, WM_CLOSE, 0, 0);
    return TRUE;
}

} // namespace

void PickerPump::request() {
    if (g_busy.exchange(true, std::memory_order_seq_cst)) return;
    if (g_worker.joinable()) g_worker.join();
    g_done.store(false, std::memory_order_seq_cst);
    g_worker = std::jthread([](std::stop_token stop) {
        g_workerThreadId.store(GetCurrentThreadId(), std::memory_order_seq_cst);
        PickResult result;
        result.status = HRESULT_FROM_WIN32(ERROR_CANCELLED);
        if (!stop.stop_requested()) result = StructureFilePicker::pick();
        {
            std::scoped_lock lock(g_mutex);
            g_answer = std::move(result);
        }
        g_workerThreadId.store(0, std::memory_order_seq_cst);
        g_done.store(true, std::memory_order_seq_cst);
    });
}

void PickerPump::deliver(StHandle* handle) {
    if (!g_done.load(std::memory_order_seq_cst)) return;
    std::optional<PickResult> answer;
    {
        std::scoped_lock lock(g_mutex);
        answer.swap(g_answer);
    }
    g_done.store(false, std::memory_order_seq_cst);
    if (g_worker.joinable()) g_worker.join();
    g_busy.store(false, std::memory_order_seq_cst);
    if (!answer) return;
    if (FAILED(answer->status) && answer->status != HRESULT_FROM_WIN32(ERROR_CANCELLED)) {
        ModLog::warn("structure picker failed", "{\"hresult\":" + hex(answer->status) + "}");
    }
    if (answer->dropped > 0) {
        ModLog::warn("structure picker dropped items", "{\"count\":" + std::to_string(answer->dropped) + "}");
    }
    if (answer->invalid > 0) {
        ModLog::warn("structure picker skipped paths that are not valid UTF-16", "{\"count\":" + std::to_string(answer->invalid) + "}");
    }
    std::vector<StStr> views;
    views.reserve(answer->paths.size());
    for (std::string const& path : answer->paths) views.push_back(RustBridge::view(path));
    StStatus status = st_files_picked(handle, views.data(), static_cast<uint32_t>(views.size()));
    if (status != ST_STATUS_OK) {
        ModLog::warn("st_files_picked failed", "{\"status\":" + std::to_string(static_cast<int>(status)) + "}");
    }
}

void PickerPump::stop() {
    if (g_worker.joinable()) {
        g_worker.request_stop();
        // the dialog is modal on the worker thread, so it is closed as a cancel before the join
        for (int attempt = 0; attempt < CLOSE_ATTEMPTS && !g_done.load(std::memory_order_seq_cst); ++attempt) {
            DWORD threadId = g_workerThreadId.load(std::memory_order_seq_cst);
            if (threadId != 0) EnumThreadWindows(threadId, closeWindow, 0);
            Sleep(CLOSE_WAIT_MS);
        }
        if (!g_done.load(std::memory_order_seq_cst)) {
            ModLog::warn("structure picker did not close; waiting for it", "{\"attempts\":" + std::to_string(CLOSE_ATTEMPTS) + "}");
        }
        g_worker.join();
    }
    {
        std::scoped_lock lock(g_mutex);
        g_answer.reset();
    }
    g_done.store(false, std::memory_order_seq_cst);
    g_busy.store(false, std::memory_order_seq_cst);
}

} // namespace stratum
