#pragma once

#include "IEventDispatcher.h"
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <deque>
#include <iostream>
#include <map>
#include <mutex>

namespace SCE::Dispatchers {

/**
 * @brief Base class for event dispatchers with shared timer and task management
 *
 * Provides common implementation for timer metadata management and task queue
 * operations. Platform-specific dispatchers inherit from this class and implement
 * the pure virtual methods for their specific event loop mechanism.
 *
 * Architecture Compliance:
 * - Zero Duplication: Shared logic for all dispatcher backends
 * - Single Source of Truth: Common timer/task management in one place
 * - Thread Safety: Protected data structures with mutex
 *
 * Derived classes must implement:
 * - start() - Initialize platform-specific event loop
 * - enqueue() - Submit task to platform event loop
 * - run() - Run platform event loop
 * - startTimerImpl() - Create platform-native timer
 * - stopTimerImpl() - Stop platform-native timer
 * - notifyDispatcherAboutEvent() - Wake up event loop
 */
class EventDispatcherBase : public IEventDispatcher {
public:
    EventDispatcherBase() : running_(false), stopRequested_(false) {}

    ~EventDispatcherBase() override = default;

    // Disable copy/move
    EventDispatcherBase(const EventDispatcherBase &) = delete;
    EventDispatcherBase &operator=(const EventDispatcherBase &) = delete;
    EventDispatcherBase(EventDispatcherBase &&) = delete;
    EventDispatcherBase &operator=(EventDispatcherBase &&) = delete;

    /**
     * @brief Stop the event dispatcher
     *
     * Signals the event loop to stop. Derived classes should call this
     * and then perform platform-specific cleanup.
     */
    void stop() override {
        stopRequested_.store(true);
        cv_.notify_all();  // Wake up run() if waiting
    }

    /**
     * @brief Check if dispatcher is running
     *
     * @return true if start() was called and stop() not yet called
     */
    bool isRunning() const override {
        return running_.load() && !stopRequested_.load();
    }

    /**
     * @brief Start a timer
     *
     * Stores timer metadata and delegates to platform-specific implementation.
     *
     * @param timerID Unique timer identifier
     * @param delayMs Delay in milliseconds before callback execution
     * @param callback Function to execute when timer expires
     * @param periodic If true, timer repeats automatically (default: false)
     *
     * @threadsafe
     */
    void startTimer(int timerID, unsigned int delayMs, std::function<void()> callback, bool periodic = false) override {
        std::lock_guard<std::mutex> lock(timerMutex_);

        auto now = std::chrono::steady_clock::now();
        auto expiryTime = now + std::chrono::milliseconds(delayMs);

        TimerInfo info{expiryTime, std::chrono::milliseconds(delayMs), std::move(callback), periodic};
        runningTimers_[timerID] = std::move(info);

        startTimerImpl(timerID, delayMs, periodic);
    }

    /**
     * @brief Stop a running timer
     *
     * Removes timer metadata and delegates to platform-specific implementation.
     *
     * @param timerID Timer identifier to stop
     *
     * @threadsafe
     */
    void stopTimer(int timerID) override {
        std::lock_guard<std::mutex> lock(timerMutex_);

        auto it = runningTimers_.find(timerID);
        if (it != runningTimers_.end()) {
            stopTimerImpl(timerID);
            runningTimers_.erase(it);
        }
    }

    /**
     * @brief Restart a running or expired timer
     *
     * Restarts the timer with the same interval and settings that were
     * provided to startTimer(). The timer must still exist in the internal
     * timer map (not yet stopped with stopTimer()).
     *
     * @param timerID Timer identifier to restart
     * @return true if timer was found and restarted, false if timer doesn't exist
     *
     * @threadsafe
     */
    bool restartTimer(int timerID) override {
        std::lock_guard<std::mutex> lock(timerMutex_);

        auto it = runningTimers_.find(timerID);
        if (it == runningTimers_.end()) {
            return false;  // Timer not found
        }

        TimerInfo &info = it->second;
        auto now = std::chrono::steady_clock::now();
        info.expiryTime = now + info.interval;

        // Stop existing platform timer and restart with same interval
        stopTimerImpl(timerID);
        startTimerImpl(timerID, static_cast<unsigned int>(info.interval.count()), info.periodic);

        return true;
    }

    /**
     * @brief Check if timer is currently running
     *
     * Returns true only if the timer is actively running (not expired).
     * For one-shot timers that have fired, returns false even though
     * timer info is kept for restartTimer() support.
     *
     * @param timerID Timer identifier to check
     * @return true if timer is active, false otherwise
     *
     * @threadsafe
     */
    bool isTimerRunning(int timerID) const override {
        return isTimerRunningImpl(timerID);
    }

    /**
     * @brief Get number of pending tasks
     *
     * @return Number of tasks in queue
     *
     * @threadsafe
     */
    size_t pendingTasks() const override {
        std::lock_guard<std::mutex> lock(mutex_);
        return taskQueue_.size();
    }

protected:
    /**
     * @brief Platform-specific timer running check
     *
     * Default implementation checks if timer exists and is not expired.
     * Override in derived classes to check native timer state.
     *
     * @param timerID Timer identifier to check
     * @return true if timer is actively running
     */
    virtual bool isTimerRunningImpl(int timerID) const {
        std::lock_guard<std::mutex> lock(timerMutex_);
        auto it = runningTimers_.find(timerID);
        if (it == runningTimers_.end()) {
            return false;
        }
        // Check if timer is expired (expiryTime set to max)
        return it->second.expiryTime != std::chrono::steady_clock::time_point::max();
    }

    /**
     * @brief Timer metadata
     */
    struct TimerInfo {
        std::chrono::steady_clock::time_point expiryTime;  ///< When timer expires
        std::chrono::milliseconds interval;                ///< Timer interval (for periodic)
        std::function<void()> callback;                    ///< Callback to execute
        bool periodic;                                     ///< If true, auto-restart
    };

    /**
     * @brief Platform-specific timer start implementation
     *
     * Called when startTimer() is invoked. Timer metadata is already stored.
     * Implementation should create platform-native timer (QTimer, g_timeout_source, etc.)
     *
     * @param timerID Timer identifier
     * @param intervalMs Timer interval in milliseconds
     * @param periodic Whether timer should repeat
     *
     * @note Called with timerMutex_ held
     */
    virtual void startTimerImpl(int timerID, unsigned int intervalMs, bool periodic) = 0;

    /**
     * @brief Platform-specific timer stop implementation
     *
     * Called when stopTimer() is invoked. Timer will be removed from metadata after this.
     * Implementation should destroy platform-native timer.
     *
     * @param timerID Timer identifier to stop
     *
     * @note Called with timerMutex_ held
     */
    virtual void stopTimerImpl(int timerID) = 0;

    /**
     * @brief Notify event loop about new event/task
     *
     * Platform-specific mechanism to wake up the event loop.
     * - StdThread: cv_.notify_one()
     * - Qt: QCoreApplication::postEvent()
     * - GLib: write to pipe
     */
    virtual void notifyDispatcherAboutEvent() = 0;

    /**
     * @brief Handle timer expiry for StdThread-like implementations
     *
     * Called when a timer expires. Returns the callback and handles
     * periodic timer rescheduling.
     *
     * @param timerID Timer that expired
     * @return Next interval for periodic timer, 0 if one-shot (removed)
     */
    unsigned int handleTimerExpiry(int timerID) {
        std::lock_guard<std::mutex> lock(timerMutex_);

        auto it = runningTimers_.find(timerID);
        if (it == runningTimers_.end()) {
            return 0;
        }

        TimerInfo &info = it->second;
        auto now = std::chrono::steady_clock::now();

        // Enqueue callback
        {
            std::lock_guard<std::mutex> taskLock(mutex_);
            taskQueue_.push_back(info.callback);
        }
        notifyDispatcherAboutEvent();

        if (info.periodic) {
            // Reschedule with drift prevention
            auto nextExpiry = info.expiryTime + info.interval;
            if (nextExpiry <= now) {
                nextExpiry = now + info.interval;
            }
            info.expiryTime = nextExpiry;
            return static_cast<unsigned int>(info.interval.count());
        } else {
            runningTimers_.erase(it);
            return 0;
        }
    }

    /**
     * @brief Get timer callback for execution
     *
     * Returns the callback for a timer without removing it.
     * Used by Qt/GLib where native timers handle scheduling.
     *
     * @param timerID Timer identifier
     * @return Callback function, or empty function if timer not found
     */
    std::function<void()> getTimerCallback(int timerID) const {
        std::lock_guard<std::mutex> lock(timerMutex_);
        auto it = runningTimers_.find(timerID);
        if (it != runningTimers_.end()) {
            return it->second.callback;
        }
        return {};
    }

    /**
     * @brief Mark timer as expired (one-shot timer fired)
     *
     * Sets expiryTime to max to indicate timer is no longer running.
     * Timer info is kept for restartTimer() support.
     *
     * @param timerID Timer identifier to mark as expired
     */
    void markTimerExpiredInternal(int timerID) {
        std::lock_guard<std::mutex> lock(timerMutex_);
        auto it = runningTimers_.find(timerID);
        if (it != runningTimers_.end()) {
            it->second.expiryTime = std::chrono::steady_clock::time_point::max();
        }
    }

    /**
     * @brief Execute pending tasks
     *
     * Processes all tasks in the queue. Called from run() in derived classes.
     *
     * @return true if tasks were executed, false if queue was empty
     */
    bool dispatchPendingTasks() {
        std::deque<std::function<void()>> tasksToExecute;

        {
            std::lock_guard<std::mutex> lock(mutex_);
            tasksToExecute.swap(taskQueue_);
        }

        for (auto &task : tasksToExecute) {
            try {
                task();
            } catch (const std::exception &e) {
                std::cerr << "EventDispatcherBase: Task execution failed: " << e.what() << std::endl;
            } catch (...) {
                std::cerr << "EventDispatcherBase: Task execution failed with unknown exception" << std::endl;
            }
        }

        return !tasksToExecute.empty();
    }

    // Shared state
    std::atomic<bool> running_;                    ///< Dispatcher running state
    std::atomic<bool> stopRequested_;              ///< Stop signal flag
    std::deque<std::function<void()>> taskQueue_;  ///< FIFO task queue
    mutable std::mutex mutex_;                     ///< Protects taskQueue_
    std::condition_variable cv_;                   ///< Notifies run() of new tasks
    std::map<int, TimerInfo> runningTimers_;       ///< Active timers
    mutable std::mutex timerMutex_;                ///< Protects runningTimers_
};

}  // namespace SCE::Dispatchers
