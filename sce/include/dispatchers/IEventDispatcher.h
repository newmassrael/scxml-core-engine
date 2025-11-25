#pragma once

#include <functional>
#include <memory>

namespace SCE::Dispatchers {

/**
 * @brief Interface for asynchronous event dispatchers
 *
 * Provides a common interface for different event loop implementations
 * (std::thread, Qt, GLib, etc.) to enable asynchronous state machine
 * event processing.
 *
 * Design Goals:
 * - Zero Duplication: Single interface for all dispatcher backends
 * - Zero Overhead: Only pay for what you use (compile-time optional)
 * - Thread Safety: Safe event submission from multiple threads
 * - Platform Agnostic: Support multiple event loop backends
 *
 * Architecture Compliance:
 * - Single Source of Truth: IEventDispatcher is the authoritative interface
 * - Helper Pattern: Dispatchers are helpers for async event processing
 * - Optional Feature: Disabled by default (no overhead if unused)
 *
 * Usage:
 * @code
 * auto dispatcher = StdThreadDispatcher::create();
 * AsyncStateMachine<MyStateMachine> sm(dispatcher);
 *
 * sm.initialize();
 * sm.postEvent(Event::Start);  // Thread-safe async event
 *
 * dispatcher->run();  // Start event loop
 * @endcode
 */
class IEventDispatcher {
public:
    virtual ~IEventDispatcher() = default;

    /**
     * @brief Start the event dispatcher
     *
     * Initializes the event loop and prepares for event processing.
     * Must be called before enqueue() can be used.
     *
     * @note Thread safety depends on implementation
     */
    virtual void start() = 0;

    /**
     * @brief Stop the event dispatcher
     *
     * Signals the event loop to stop. Pending tasks may or may not
     * be processed depending on implementation.
     *
     * @note Thread safety depends on implementation
     */
    virtual void stop() = 0;

    /**
     * @brief Check if dispatcher is running
     *
     * @return true if dispatcher is running, false otherwise
     *
     * @threadsafe Implementation must be thread-safe
     */
    virtual bool isRunning() const = 0;

    /**
     * @brief Enqueue a task for execution on dispatcher thread
     *
     * Submits a task to be executed asynchronously on the dispatcher's
     * event loop thread. The task will be executed in FIFO order with
     * other enqueued tasks.
     *
     * @param task Function to execute on dispatcher thread
     *
     * @threadsafe Implementation must be thread-safe (can be called from any thread)
     */
    virtual void enqueue(std::function<void()> task) = 0;

    /**
     * @brief Run the event loop (blocking)
     *
     * Starts processing events and blocks until stop() is called.
     * This is the main event loop function.
     *
     * @note Should be called from the same thread that created the dispatcher
     */
    virtual void run() = 0;

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
     * @threadsafe Implementation must be thread-safe
     *
     * @example
     * @code
     * dispatcher->startTimer(1, 1000, []() {
     *     std::cout << "Timer expired\n";
     * }, true);  // Fires every 1000ms
     * @endcode
     */
    virtual void startTimer(int timerID, unsigned int delayMs, std::function<void()> callback,
                            bool periodic = false) = 0;

    /**
     * @brief Stop a running timer
     *
     * Cancels the timer if it exists. Has no effect if timer is not running.
     *
     * @param timerID Timer identifier to stop
     *
     * @threadsafe Implementation must be thread-safe
     */
    virtual void stopTimer(int timerID) = 0;

    /**
     * @brief Check if timer is currently running
     *
     * @param timerID Timer identifier to check
     * @return true if timer is active, false otherwise
     *
     * @threadsafe Implementation must be thread-safe
     */
    virtual bool isTimerRunning(int timerID) const = 0;
};

}  // namespace SCE::Dispatchers
