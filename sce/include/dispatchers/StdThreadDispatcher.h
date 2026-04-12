// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "EventDispatcherBase.h"
#include <thread>

namespace SCE::Dispatchers {

/**
 * @brief std::thread-based event dispatcher implementation
 *
 * Implements asynchronous event processing using std::thread,
 * std::mutex, and std::condition_variable. Provides a simple
 * event loop for processing tasks asynchronously.
 *
 * Thread Safety:
 * - enqueue() is thread-safe (can be called from any thread)
 * - start()/stop()/run() should be called from dispatcher thread
 *
 * Architecture Compliance:
 * - Zero Duplication: Inherits common logic from EventDispatcherBase
 * - Single Source of Truth: Implements IEventDispatcher interface
 * - Thread Safety: Conditional mutex protection for task queue
 *
 * Example:
 * @code
 * auto dispatcher = StdThreadDispatcher::create();
 * dispatcher->start();
 *
 * // From any thread
 * dispatcher->enqueue([]() {
 *     std::cout << "Task executed on dispatcher thread\n";
 * });
 *
 * // Run event loop (blocking)
 * std::thread eventLoop([dispatcher]() {
 *     dispatcher->run();
 * });
 *
 * // Later: stop dispatcher
 * dispatcher->stop();
 * eventLoop.join();
 * @endcode
 */
class StdThreadDispatcher : public EventDispatcherBase {
public:
    /**
     * @brief Create shared_ptr instance of StdThreadDispatcher
     *
     * @return Shared pointer to new dispatcher instance
     */
    static std::shared_ptr<StdThreadDispatcher> create() {
        return std::make_shared<StdThreadDispatcher>();
    }

    /**
     * @brief Constructor
     */
    StdThreadDispatcher() = default;

    /**
     * @brief Destructor - stops dispatcher if still running
     */
    ~StdThreadDispatcher() override {
        // Always signal stop to all threads
        stop();

        // Always join timer thread if it was started
        if (timerThread_.joinable()) {
            timerThread_.join();
        }
    }

    /**
     * @brief Start the event dispatcher
     *
     * Marks dispatcher as ready to accept tasks and starts timer thread.
     * Must be called before enqueue() or run().
     */
    void start() override {
        // Join existing timer thread if still running
        if (timerThread_.joinable()) {
            stopRequested_.store(true);
            cvTimer_.notify_all();
            timerThread_.join();
        }

        running_.store(true);
        stopRequested_.store(false);
        timerChanged_.store(false);

        // Start timer polling thread
        timerThread_ = std::thread([this]() { handleTimers(); });
    }

    /**
     * @brief Stop the event dispatcher
     *
     * Signals the event loop and timer thread to stop.
     * Pending tasks will not be processed.
     * The run() method will return after processing current task.
     */
    void stop() override {
        EventDispatcherBase::stop();  // Sets stopRequested_ and notifies cv_
        cvTimer_.notify_all();        // Wake up timer thread
    }

    /**
     * @brief Enqueue a task for execution on dispatcher thread
     *
     * Thread-safe: Can be called from any thread.
     *
     * @param task Function to execute on dispatcher thread
     *
     * @throws std::runtime_error if dispatcher not started
     */
    void enqueue(std::function<void()> task) override {
        if (!running_.load()) {
            throw std::runtime_error("StdThreadDispatcher: Cannot enqueue task, dispatcher not started");
        }

        {
            std::lock_guard<std::mutex> lock(mutex_);
            taskQueue_.push_back(std::move(task));
        }

        notifyDispatcherAboutEvent();
    }

    /**
     * @brief Run the event loop (blocking)
     *
     * Processes tasks from queue until stop() is called.
     * Blocks waiting for tasks when queue is empty.
     *
     * @note Should be called from dispatcher thread
     */
    void run() override {
        if (!running_.load()) {
            throw std::runtime_error("StdThreadDispatcher: Cannot run, dispatcher not started");
        }

        // Store thread ID for reference
        eventLoopThreadId_ = std::this_thread::get_id();

        while (!stopRequested_.load()) {
            std::function<void()> task;

            {
                std::unique_lock<std::mutex> lock(mutex_);

                // Wait for task or stop signal
                cv_.wait(lock, [this]() { return !taskQueue_.empty() || stopRequested_.load(); });

                // Check stop condition after wakeup
                if (stopRequested_.load()) {
                    break;
                }

                // Pop task from queue
                if (!taskQueue_.empty()) {
                    task = std::move(taskQueue_.front());
                    taskQueue_.pop_front();
                }
            }

            // Execute task outside lock
            if (task) {
                try {
                    task();
                } catch (const std::exception &e) {
                    // Log error to stderr and continue processing
                    // No dependency on logging library for zero overhead
                    std::cerr << "StdThreadDispatcher: Task execution failed: " << e.what() << std::endl;
                } catch (...) {
                    std::cerr << "StdThreadDispatcher: Task execution failed with unknown exception" << std::endl;
                }
            }
        }

        running_.store(false);
    }

protected:
    /**
     * @brief Start platform-specific timer
     *
     * Signals timer thread to wake up and check for the new timer.
     * Sets timerChanged_ flag so the wait_for predicate returns true.
     */
    void startTimerImpl(int /*timerID*/, unsigned int /*intervalMs*/, bool /*periodic*/) override {
        timerChanged_.store(true);
        cvTimer_.notify_one();
    }

    /**
     * @brief Stop platform-specific timer
     *
     * Timer is removed from runningTimers_ by base class.
     * No additional action needed for std::thread implementation.
     */
    void stopTimerImpl(int /*timerID*/) override {
        // No-op: Timer removed from map by base class
        // Timer thread will skip it on next check
    }

    /**
     * @brief Notify event loop about new task
     */
    void notifyDispatcherAboutEvent() override {
        cv_.notify_one();
    }

private:
    /**
     * @brief Timer polling thread main loop
     *
     * Checks for expired timers and enqueues their callbacks.
     * Runs until stop() is called.
     */
    void handleTimers() {
        while (!stopRequested_.load()) {
            std::unique_lock<std::mutex> lock(timerMutex_);

            // Find next timer expiry
            auto now = std::chrono::steady_clock::now();
            std::chrono::milliseconds waitTime = std::chrono::milliseconds(100);  // Default poll interval

            if (!runningTimers_.empty()) {
                // Find earliest expiry time
                auto earliestTimer =
                    std::min_element(runningTimers_.begin(), runningTimers_.end(), [](const auto &a, const auto &b) {
                        return a.second.expiryTime < b.second.expiryTime;
                    });

                auto timeUntilExpiry =
                    std::chrono::duration_cast<std::chrono::milliseconds>(earliestTimer->second.expiryTime - now);

                if (timeUntilExpiry.count() <= 0) {
                    // Timer expired - process it
                    int timerID = earliestTimer->first;
                    TimerInfo info = earliestTimer->second;

                    // Mark one-shot timer as expired BEFORE releasing timerMutex_
                    // to prevent race condition where isTimerRunning() sees stale expiryTime
                    // during the lock gap between unlock() and re-lock()
                    if (!info.periodic) {
                        earliestTimer->second.expiryTime = std::chrono::steady_clock::time_point::max();
                    }

                    // Enqueue callback on dispatcher thread
                    lock.unlock();  // Release timer lock before enqueueing
                    try {
                        enqueue(info.callback);
                    } catch (const std::exception &e) {
                        std::cerr << "StdThreadDispatcher: Failed to enqueue timer callback: " << e.what() << std::endl;
                    }
                    lock.lock();  // Re-acquire lock

                    // Handle periodic timer rescheduling
                    if (info.periodic) {
                        auto it = runningTimers_.find(timerID);
                        if (it != runningTimers_.end()) {
                            auto &timer = it->second;

                            // Calculate next expiry based on original expiry time (drift prevention)
                            auto nextExpiry = info.expiryTime + info.interval;

                            // Catch-up prevention: If processing took longer than interval,
                            // schedule from current time to avoid busy loop
                            if (nextExpiry <= now) {
                                nextExpiry = now + info.interval;
                            }

                            timer.expiryTime = nextExpiry;
                        }
                    }

                    continue;  // Recheck timers immediately
                } else {
                    waitTime = timeUntilExpiry;
                    if (waitTime > std::chrono::milliseconds(100)) {
                        waitTime = std::chrono::milliseconds(100);  // Cap wait time
                    }
                }
            }

            // Wait for next timer, new timer registration, or stop signal
            cvTimer_.wait_for(lock, waitTime, [this]() {
                return stopRequested_.load() || timerChanged_.load();
            });
            timerChanged_.store(false);
        }
    }

    std::thread timerThread_;              ///< Timer polling thread
    std::condition_variable cvTimer_;      ///< Notifies timer thread
    std::atomic<bool> timerChanged_{false};  ///< Signals new timer registration to wake wait_for predicate
    std::thread::id eventLoopThreadId_;    ///< Event loop thread ID
};

}  // namespace SCE::Dispatchers
