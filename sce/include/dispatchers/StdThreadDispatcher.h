#pragma once

#include "IEventDispatcher.h"
#include <atomic>
#include <condition_variable>
#include <deque>
#include <iostream>
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
        if (running_.load()) {
            stop();
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
     * Marks dispatcher as ready to accept tasks.
     * Must be called before enqueue() or run().
     */
    void start() override {
        running_.store(true);
        stopRequested_.store(false);
    }

    /**
     * @brief Stop the event dispatcher
     *
     * Signals the event loop to stop. Pending tasks will not be processed.
     * The run() method will return after processing current task.
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

private:
    std::atomic<bool> running_;                    ///< Dispatcher running state
    std::atomic<bool> stopRequested_;              ///< Stop signal flag
    std::deque<std::function<void()>> taskQueue_;  ///< FIFO task queue
    mutable std::mutex mutex_;                     ///< Protects taskQueue_
    std::condition_variable cv_;                   ///< Notifies run() of new tasks
};

}  // namespace SCE::Dispatchers
