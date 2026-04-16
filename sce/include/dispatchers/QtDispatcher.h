// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "EventDispatcherBase.h"

// Forward declarations to avoid Qt include in header
class QObject;
class QEvent;
class QTimer;

namespace SCE::Dispatchers {

// Forward declaration of implementation class
class QtDispatcherImpl;

/**
 * @brief Qt-based event dispatcher implementation
 *
 * Implements asynchronous event processing using Qt's event loop.
 * Uses QCoreApplication::postEvent() for task notification and
 * QTimer for timer management.
 *
 * Thread Safety:
 * - enqueue() is thread-safe (can be called from any thread)
 * - start()/stop()/run() should be called from Qt main thread
 *
 * Architecture Compliance:
 * - Zero Duplication: Inherits common logic from EventDispatcherBase
 * - Single Source of Truth: Implements IEventDispatcher interface
 * - Platform Integration: Uses native Qt event mechanisms
 *
 * Example:
 * @code
 * // In main.cpp with QCoreApplication
 * QCoreApplication app(argc, argv);
 *
 * auto dispatcher = QtDispatcher::create();
 * dispatcher->start();
 *
 * // From any thread
 * dispatcher->enqueue([]() {
 *     std::cout << "Task executed on Qt event loop\n";
 * });
 *
 * // Qt event loop processes tasks
 * app.exec();  // Or use dispatcher->run()
 * @endcode
 *
 * @note Requires Qt5/Qt6 Core module
 * @note Must have QCoreApplication instance before use
 */
class QtDispatcher : public EventDispatcherBase {
    friend class QtDispatcherImpl;  // Allow access to protected base class members

public:
    /**
     * @brief Create shared_ptr instance of QtDispatcher
     *
     * @return Shared pointer to new dispatcher instance
     */
    static std::shared_ptr<QtDispatcher> create();

    /**
     * @brief Constructor
     */
    QtDispatcher();

    /**
     * @brief Destructor - stops dispatcher if still running
     */
    ~QtDispatcher() override;

    // Disable copy/move
    QtDispatcher(const QtDispatcher &) = delete;
    QtDispatcher &operator=(const QtDispatcher &) = delete;
    QtDispatcher(QtDispatcher &&) = delete;
    QtDispatcher &operator=(QtDispatcher &&) = delete;

    /**
     * @brief Start the event dispatcher
     *
     * Registers custom Qt event type and prepares for event processing.
     * Must be called before enqueue().
     *
     * @note Requires QCoreApplication instance to exist
     */
    void start() override;

    /**
     * @brief Stop the event dispatcher
     *
     * Cleans up all timers and stops processing events.
     */
    void stop() override;

    /**
     * @brief Enqueue a task for execution on Qt event loop
     *
     * Thread-safe: Can be called from any thread.
     * Uses QCoreApplication::postEvent() for thread-safe delivery.
     *
     * @param task Function to execute on Qt event loop
     *
     * @throws std::runtime_error if dispatcher not started
     */
    void enqueue(std::function<void()> task) override;

    /**
     * @brief Run the Qt event loop (blocking)
     *
     * Starts Qt event loop using QCoreApplication::exec().
     * Tasks are processed via customEvent().
     *
     * @note Alternatively, use app.exec() directly after start()
     */
    void run() override;

    /**
     * @brief Get the Qt implementation object
     *
     * Provides access to the QObject-derived implementation for
     * advanced Qt integration (e.g., signal/slot connections).
     *
     * @return Pointer to QtDispatcherImpl, or nullptr if not started
     */
    QtDispatcherImpl *getImpl() const;

    /**
     * @brief Dispatch pending tasks (public accessor for QtDispatcherImpl)
     *
     * @return true if tasks were executed
     */
    bool processPendingTasks() {
        return dispatchPendingTasks();
    }

    /**
     * @brief Get timer callback (public accessor for QtDispatcherImpl)
     *
     * @param timerID Timer identifier
     * @return Callback function
     */
    std::function<void()> getCallback(int timerID) const {
        return getTimerCallback(timerID);
    }

    /**
     * @brief Mark timer as expired (public accessor for QtDispatcherImpl)
     *
     * @param timerID Timer identifier to mark as expired
     */
    void markTimerExpired(int timerID) {
        markTimerExpiredInternal(timerID);
    }

protected:
    void startTimerImpl(int timerID, unsigned int intervalMs, bool periodic) override;
    void stopTimerImpl(int timerID) override;
    void notifyDispatcherAboutEvent() override;

private:
    class Impl;
    std::unique_ptr<Impl> pImpl_;
};

}  // namespace SCE::Dispatchers
