// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "EventDispatcherBase.h"
#include <glib.h>

namespace SCE::Dispatchers {

/**
 * @brief GLib-based event dispatcher implementation
 *
 * Implements asynchronous event processing using GLib's main loop.
 * Uses pipe-based notification to wake up the GLib event loop and
 * g_timeout_source for timer management.
 *
 * Thread Safety:
 * - enqueue() is thread-safe (can be called from any thread)
 * - start()/stop()/run() should be called from GLib main context
 *
 * Architecture Compliance:
 * - Zero Duplication: Inherits common logic from EventDispatcherBase
 * - Single Source of Truth: Implements IEventDispatcher interface
 * - Platform Integration: Uses native GLib event mechanisms
 *
 * Example:
 * @code
 * // Create dispatcher with default GLib context
 * auto dispatcher = GLibDispatcher::create();
 * dispatcher->start();
 *
 * // From any thread
 * dispatcher->enqueue([]() {
 *     g_print("Task executed on GLib main loop\n");
 * });
 *
 * // GLib main loop processes tasks
 * dispatcher->run();  // Or use g_main_loop_run() directly
 * @endcode
 *
 * @note Requires glib-2.0 library
 */
class GLibDispatcher : public EventDispatcherBase {
public:
    /**
     * @brief Create shared_ptr instance with default GLib context
     *
     * @return Shared pointer to new dispatcher instance
     */
    static std::shared_ptr<GLibDispatcher> create();

    /**
     * @brief Create shared_ptr instance with custom GLib context
     *
     * @param context Custom GLib main context (can be nullptr for default)
     * @return Shared pointer to new dispatcher instance
     */
    static std::shared_ptr<GLibDispatcher> create(GMainContext *context);

    /**
     * @brief Constructor with default context
     */
    GLibDispatcher();

    /**
     * @brief Constructor with custom context
     *
     * @param context Custom GLib main context (can be nullptr for default)
     */
    explicit GLibDispatcher(GMainContext *context);

    /**
     * @brief Destructor - stops dispatcher and cleans up GLib resources
     */
    ~GLibDispatcher() override;

    // Disable copy/move
    GLibDispatcher(const GLibDispatcher &) = delete;
    GLibDispatcher &operator=(const GLibDispatcher &) = delete;
    GLibDispatcher(GLibDispatcher &&) = delete;
    GLibDispatcher &operator=(GLibDispatcher &&) = delete;

    /**
     * @brief Start the event dispatcher
     *
     * Creates pipe and GIOChannel for event notification.
     * Must be called before enqueue().
     */
    void start() override;

    /**
     * @brief Stop the event dispatcher
     *
     * Cleans up all timers, closes pipe, and releases GLib resources.
     */
    void stop() override;

    /**
     * @brief Enqueue a task for execution on GLib main loop
     *
     * Thread-safe: Can be called from any thread.
     * Writes to pipe to wake up GLib main loop.
     *
     * @param task Function to execute on GLib main loop
     *
     * @throws std::runtime_error if dispatcher not started
     */
    void enqueue(std::function<void()> task) override;

    /**
     * @brief Run the GLib main loop (blocking)
     *
     * Creates and runs a GMainLoop until stop() is called.
     * Tasks are processed via pipe callback.
     */
    void run() override;

protected:
    void startTimerImpl(int timerID, unsigned int intervalMs, bool periodic) override;
    void stopTimerImpl(int timerID) override;
    void notifyDispatcherAboutEvent() override;
    bool isTimerRunningImpl(int timerID) const override;

private:
    /**
     * @brief Timer callback data structure
     */
    struct TimerData {
        GLibDispatcher *dispatcher;
        int timerID;
    };

    /**
     * @brief GIOChannel callback for pipe data
     */
    static gboolean onPipeDataAvailable(GIOChannel *channel, GIOCondition condition, gpointer data);

    /**
     * @brief GLib timer callback
     */
    static gboolean onTimerEvent(gpointer data);

    /**
     * @brief Cleanup callback for timer data
     */
    static void onFreeTimerData(gpointer data);

    /**
     * @brief Unregister all active timers
     */
    void unregisterAllTimers();

    GMainContext *context_ = nullptr;        ///< GLib main context
    GMainLoop *mainLoop_ = nullptr;          ///< GLib main loop (for run())
    GIOChannel *readChannel_ = nullptr;      ///< Pipe read channel
    GSource *ioSource_ = nullptr;            ///< IO source for pipe
    int pipeFD_[2] = {-1, -1};               ///< Pipe file descriptors
    std::mutex pipeMutex_;                   ///< Protects pipe writes
    std::map<int, GSource *> nativeTimers_;  ///< Active GLib timers
    mutable std::mutex nativeTimersMutex_;   ///< Protects nativeTimers_ access
};

}  // namespace SCE::Dispatchers
