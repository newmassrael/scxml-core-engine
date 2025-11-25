#include "dispatchers/GLibDispatcher.h"

#include <iostream>
#include <unistd.h>

namespace SCE::Dispatchers {

std::shared_ptr<GLibDispatcher> GLibDispatcher::create() {
    return std::make_shared<GLibDispatcher>();
}

std::shared_ptr<GLibDispatcher> GLibDispatcher::create(GMainContext *context) {
    return std::make_shared<GLibDispatcher>(context);
}

GLibDispatcher::GLibDispatcher() : context_(nullptr) {}

GLibDispatcher::GLibDispatcher(GMainContext *context) : context_(context) {}

GLibDispatcher::~GLibDispatcher() {
    stop();
}

void GLibDispatcher::start() {
    // Already started check
    if (pipeFD_[0] != -1) {
        return;
    }

    // Create pipe for notification
    if (pipe(pipeFD_) != 0) {
        throw std::runtime_error("GLibDispatcher: Failed to create pipe");
    }

    // Create GIO channel from read end of pipe
    readChannel_ = g_io_channel_unix_new(pipeFD_[0]);
    if (!readChannel_) {
        close(pipeFD_[0]);
        close(pipeFD_[1]);
        pipeFD_[0] = -1;
        pipeFD_[1] = -1;
        throw std::runtime_error("GLibDispatcher: Failed to create GIOChannel");
    }

    // Create watch source for the channel
    ioSource_ = g_io_create_watch(readChannel_, static_cast<GIOCondition>(G_IO_IN | G_IO_HUP));
    if (!ioSource_) {
        g_io_channel_unref(readChannel_);
        readChannel_ = nullptr;
        close(pipeFD_[0]);
        close(pipeFD_[1]);
        pipeFD_[0] = -1;
        pipeFD_[1] = -1;
        throw std::runtime_error("GLibDispatcher: Failed to create IO source");
    }

    // Set callback and attach to context
    g_source_set_callback(ioSource_, reinterpret_cast<GSourceFunc>(onPipeDataAvailable), this, nullptr);
    g_source_attach(ioSource_, context_);

    running_.store(true);
    stopRequested_.store(false);
}

void GLibDispatcher::stop() {
    // Set running_ = false early to handle race with run()
    running_.store(false);

    EventDispatcherBase::stop();

    // Wake up the main loop to check stopRequested_
    notifyDispatcherAboutEvent();

    // Stop main loop if running
    if (mainLoop_) {
        g_main_loop_quit(mainLoop_);
    }

    unregisterAllTimers();

    // Cleanup IO source
    if (ioSource_) {
        g_source_destroy(ioSource_);
        g_source_unref(ioSource_);
        ioSource_ = nullptr;
    }

    // Cleanup channel and pipe
    if (readChannel_) {
        g_io_channel_unref(readChannel_);
        readChannel_ = nullptr;

        close(pipeFD_[0]);
        close(pipeFD_[1]);
        pipeFD_[0] = -1;
        pipeFD_[1] = -1;
    }
    // running_.store(false) already called at start of stop()
}

void GLibDispatcher::enqueue(std::function<void()> task) {
    if (!running_.load()) {
        throw std::runtime_error("GLibDispatcher: Cannot enqueue task, dispatcher not started");
    }

    {
        std::lock_guard<std::mutex> lock(mutex_);
        taskQueue_.push_back(std::move(task));
    }

    notifyDispatcherAboutEvent();
}

void GLibDispatcher::run() {
    // Check if dispatcher is valid for running
    // Allow early exit if stop() was called before run() started (race condition)
    if (!running_.load()) {
        if (stopRequested_.load()) {
            // stop() was called before run() - graceful exit
            return;
        }
        throw std::runtime_error("GLibDispatcher: Cannot run, dispatcher not started");
    }

    // Use iteration-based approach for better stop() responsiveness
    // This avoids race condition where stop() is called before mainLoop_ is created
    GMainContext *ctx = context_ ? context_ : g_main_context_default();

    while (!stopRequested_.load()) {
        // Process pending events (blocking)
        // Use may_block=TRUE but check stopRequested_ frequently
        g_main_context_iteration(ctx, TRUE);
    }

    running_.store(false);
}

void GLibDispatcher::startTimerImpl(int timerID, unsigned int intervalMs, bool periodic) {
    std::lock_guard<std::mutex> lock(nativeTimersMutex_);

    // If timer already exists, stop it first (replace behavior)
    auto it = nativeTimers_.find(timerID);
    if (it != nativeTimers_.end()) {
        g_source_destroy(it->second);
        g_source_unref(it->second);
        nativeTimers_.erase(it);
    }

    // Create timer data
    TimerData *timerData = new TimerData{this, timerID};

    // Create timeout source
    GSource *timeoutSource = g_timeout_source_new(intervalMs);

    // Set callback with cleanup function
    g_source_set_callback(timeoutSource, onTimerEvent, timerData, onFreeTimerData);

    // Attach to context
    g_source_attach(timeoutSource, context_);

    // Store reference
    nativeTimers_[timerID] = timeoutSource;
}

void GLibDispatcher::stopTimerImpl(int timerID) {
    std::lock_guard<std::mutex> lock(nativeTimersMutex_);

    auto it = nativeTimers_.find(timerID);
    if (it != nativeTimers_.end()) {
        g_source_destroy(it->second);
        g_source_unref(it->second);
        nativeTimers_.erase(it);
    }
}

void GLibDispatcher::notifyDispatcherAboutEvent() {
    std::lock_guard<std::mutex> lock(pipeMutex_);
    if (pipeFD_[1] >= 0) {
        char dummy = 1;
        // Write to pipe to wake up GLib main loop
        ssize_t written = write(pipeFD_[1], &dummy, sizeof(dummy));
        (void)written;  // Ignore return value
    }
}

void GLibDispatcher::unregisterAllTimers() {
    std::lock_guard<std::mutex> lock(nativeTimersMutex_);

    for (auto &pair : nativeTimers_) {
        g_source_destroy(pair.second);
        g_source_unref(pair.second);
    }
    nativeTimers_.clear();
}

gboolean GLibDispatcher::onPipeDataAvailable(GIOChannel *channel, GIOCondition condition, gpointer data) {
    GLibDispatcher *dispatcher = static_cast<GLibDispatcher *>(data);

    if (dispatcher->stopRequested_.load()) {
        return FALSE;  // Stop watching
    }

    if (!(condition & G_IO_HUP)) {
        // Read and discard the notification byte
        char dummy;
        ssize_t bytes = read(dispatcher->pipeFD_[0], &dummy, sizeof(dummy));
        (void)bytes;  // Ignore return value

        // Dispatch pending tasks
        dispatcher->dispatchPendingTasks();
    }

    // Check if stop was requested during dispatching
    if (dispatcher->stopRequested_.load()) {
        return FALSE;
    }

    return TRUE;  // Continue watching
}

gboolean GLibDispatcher::onTimerEvent(gpointer data) {
    TimerData *timerData = static_cast<TimerData *>(data);
    if (!timerData || !timerData->dispatcher) {
        return FALSE;
    }

    GLibDispatcher *dispatcher = timerData->dispatcher;
    int timerID = timerData->timerID;

    // Execute callback with exception safety
    if (auto callback = dispatcher->getTimerCallback(timerID)) {
        try {
            callback();
        } catch (const std::exception &e) {
            std::cerr << "GLibDispatcher: Timer callback exception: " << e.what() << std::endl;
        } catch (...) {
            std::cerr << "GLibDispatcher: Timer callback unknown exception" << std::endl;
        }
    }

    // Check if periodic timer - early return if should continue
    {
        std::lock_guard<std::mutex> lock(dispatcher->timerMutex_);
        auto it = dispatcher->runningTimers_.find(timerID);
        if (it != dispatcher->runningTimers_.end() && it->second.periodic) {
            return TRUE;  // Continue periodic timer
        }
        // One-shot timer - remove from runningTimers_
        if (it != dispatcher->runningTimers_.end()) {
            dispatcher->runningTimers_.erase(it);
        }
    }

    // One-shot timer cleanup - remove from native timers
    // Note: Don't unref here - GLib does it when returning FALSE
    {
        std::lock_guard<std::mutex> lock(dispatcher->nativeTimersMutex_);
        dispatcher->nativeTimers_.erase(timerID);
    }

    return FALSE;
}

void GLibDispatcher::onFreeTimerData(gpointer data) {
    delete static_cast<TimerData *>(data);
}

}  // namespace SCE::Dispatchers
