#pragma once

#include "IEventDispatcher.h"
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <deque>
#include <iostream>
#include <map>
#include <mutex>
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
 * - Zero Duplication: Uses EventQueueManager-like pattern for task queue
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
class StdThreadDispatcher : public IEventDispatcher {
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
    StdThreadDispatcher() : running_(false), stopRequested_(false) {}

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

    // Disable copy/move
    StdThreadDispatcher(const StdThreadDispatcher &) = delete;
    StdThreadDispatcher &operator=(const StdThreadDispatcher &) = delete;
    StdThreadDispatcher(StdThreadDispatcher &&) = delete;
    StdThreadDispatcher &operator=(StdThreadDispatcher &&) = delete;

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
        stopRequested_.store(true);
        cv_.notify_all();       // Wake up run() if waiting
        cvTimer_.notify_all();  // Wake up timer thread
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

        cv_.notify_one();  // Wake up run() to process task
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

    /**
     * @brief Get number of pending tasks
     *
     * @return Number of tasks in queue
     *
     * @threadsafe
     */
    size_t pendingTasks() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return taskQueue_.size();
    }

    /**
     * @brief Start a timer
     *
     * Schedules a callback to be executed after the specified delay.
     * If timer with this ID already exists, it will be replaced.
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

        cvTimer_.notify_one();  // Wake up timer thread
    }

    /**
     * @brief Stop a running timer
     *
     * Cancels the timer if it exists. Has no effect if timer is not running.
     *
     * @param timerID Timer identifier to stop
     *
     * @threadsafe
     */
    void stopTimer(int timerID) override {
        std::lock_guard<std::mutex> lock(timerMutex_);
        runningTimers_.erase(timerID);
    }

    /**
     * @brief Check if timer is currently running
     *
     * @param timerID Timer identifier to check
     * @return true if timer is active, false otherwise
     *
     * @threadsafe
     */
    bool isTimerRunning(int timerID) const override {
        std::lock_guard<std::mutex> lock(timerMutex_);
        return runningTimers_.find(timerID) != runningTimers_.end();
    }

private:
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
                    // Timer expired - enqueue callback
                    int timerID = earliestTimer->first;
                    TimerInfo info = earliestTimer->second;

                    // Enqueue callback on dispatcher thread
                    lock.unlock();  // Release timer lock before enqueueing
                    try {
                        enqueue(info.callback);
                    } catch (const std::exception &e) {
                        std::cerr << "StdThreadDispatcher: Failed to enqueue timer callback: " << e.what() << std::endl;
                    }
                    lock.lock();  // Re-acquire lock

                    // Handle periodic timer or remove one-shot
                    if (info.periodic) {
                        // Reschedule periodic timer with drift prevention
                        auto &timer = runningTimers_[timerID];

                        // Calculate next expiry based on original expiry time (drift prevention)
                        auto nextExpiry = earliestTimer->second.expiryTime + info.interval;

                        // Catch-up prevention: If processing took longer than interval,
                        // schedule from current time to avoid busy loop
                        if (nextExpiry <= now) {
                            nextExpiry = now + info.interval;
                        }

                        timer.expiryTime = nextExpiry;
                    } else {
                        // Remove one-shot timer
                        runningTimers_.erase(timerID);
                    }

                    continue;  // Recheck timers immediately
                } else {
                    waitTime = timeUntilExpiry;
                    if (waitTime > std::chrono::milliseconds(100)) {
                        waitTime = std::chrono::milliseconds(100);  // Cap wait time
                    }
                }
            }

            // Wait for next timer or stop signal
            cvTimer_.wait_for(lock, waitTime, [this]() { return stopRequested_.load(); });
        }
    }

    std::atomic<bool> running_;                    ///< Dispatcher running state
    std::atomic<bool> stopRequested_;              ///< Stop signal flag
    std::deque<std::function<void()>> taskQueue_;  ///< FIFO task queue
    mutable std::mutex mutex_;                     ///< Protects taskQueue_
    std::condition_variable cv_;                   ///< Notifies run() of new tasks
    std::thread timerThread_;                      ///< Timer polling thread
    std::map<int, TimerInfo> runningTimers_;       ///< Active timers
    mutable std::mutex timerMutex_;                ///< Protects runningTimers_
    std::condition_variable cvTimer_;              ///< Notifies timer thread
    std::thread::id eventLoopThreadId_;            ///< Event loop thread ID
};

}  // namespace SCE::Dispatchers
