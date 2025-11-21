#pragma once

#include "IEventDispatcher.h"
#include "IEventTarget.h"
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <queue>
#include <shared_mutex>  // TSAN FIX: For reader-writer lock pattern
#include <string>
#include <thread>
#include <unordered_map>
#include <vector>

namespace SCE {

/**
 * @brief Concrete implementation of IEventScheduler
 *
 * This implementation provides thread-safe event scheduling using a dedicated
 * timer thread and condition variables for precise timing. It maintains an
 * internal map of scheduled events and automatically generates unique send IDs.
 *
 * Key features:
 * - Thread-safe operations with mutex protection
 * - Precise timing using std::chrono and condition variables
 * - Automatic send ID generation with collision avoidance
 * - Proper resource cleanup on shutdown
 * - W3C SCXML compliant behavior for duplicate send IDs
 */
class EventSchedulerImpl : public IEventScheduler {
public:
    /**
     * @brief Construct scheduler with execution callback
     *
     * @param executionCallback Callback to invoke when events are ready for execution
     */
    explicit EventSchedulerImpl(EventExecutionCallback executionCallback);

    /**
     * @brief Destructor - automatically shuts down scheduler
     */
    virtual ~EventSchedulerImpl();

    // IEventScheduler implementation
    std::future<std::string> scheduleEvent(const EventDescriptor &event, std::chrono::milliseconds delay,
                                           std::shared_ptr<IEventTarget> target, const std::string &sendId = "",
                                           const std::string &sessionId = "") override;

    bool cancelEvent(const std::string &sendId, const std::string &sessionId = "") override;
    size_t cancelEventsForSession(const std::string &sessionId) override;
    bool hasEvent(const std::string &sendId) const override;
    size_t getScheduledEventCount() const override;
    void shutdown(bool waitForCompletion = true) override;
    bool isRunning() const override;
    std::vector<ScheduledEventInfo> getScheduledEvents() const override;
    void setMode(SchedulerMode mode) override;
    SchedulerMode getMode() const override;
    size_t forcePoll() override;

#ifdef __EMSCRIPTEN__
    /**
     * @brief Poll for ready events and execute them (WASM only)
     *
     * W3C SCXML 6.2: In WASM environments without timer threads, this method
     * is called periodically to check for scheduled events whose execution time
     * has arrived and executes them synchronously on the calling thread.
     *
     * Native builds use automatic timer threads for scheduled event processing.
     *
     * @return Number of events that were processed
     */
    size_t poll();
#endif

private:
    /**
     * @brief Internal structure for scheduled events with priority queue support
     */
    struct ScheduledEvent {
        EventDescriptor event;
        std::chrono::steady_clock::time_point executeAt;
        std::chrono::milliseconds originalDelay;  // Original delay for step backward restoration
        std::shared_ptr<IEventTarget> target;
        std::promise<std::string> sendIdPromise;
        std::string sendId;
        std::string sessionId;
        uint64_t sequenceNumber = 0;  // FIFO ordering for events with same executeAt
        bool cancelled = false;

        // W3C SCXML 3.13: Logical time for MANUAL mode deterministic stepping
        // In MANUAL mode, events are processed based on logical step sequence, not real-time
        std::chrono::milliseconds logicalExecuteTime{0};

        ScheduledEvent(const EventDescriptor &evt, std::chrono::steady_clock::time_point execTime,
                       std::chrono::milliseconds origDelay, std::shared_ptr<IEventTarget> tgt, const std::string &id,
                       const std::string &sessId, uint64_t seqNum)
            : event(evt), executeAt(execTime), originalDelay(origDelay), target(std::move(tgt)), sendId(id),
              sessionId(sessId), sequenceNumber(seqNum) {}
    };

    /**
     * @brief Comparator for priority queue (earlier times have higher priority, FIFO for same time)
     * CRITICAL FIX: Use shared_ptr instead of raw pointers for memory safety
     */
    struct ExecutionTimeComparator {
        bool operator()(const std::shared_ptr<ScheduledEvent> &a, const std::shared_ptr<ScheduledEvent> &b) const {
            // For min-heap, we want earlier times to have higher priority
            // std::priority_queue is max-heap by default, so we reverse the comparison
            if (a->executeAt != b->executeAt) {
                return a->executeAt > b->executeAt;
            }
            // FIFO ordering: lower sequence number = higher priority (earlier in queue)
            return a->sequenceNumber > b->sequenceNumber;
        }
    };

    /**
     * @brief Process all events that are ready for execution
     *
     * Called by the timer thread (Native) or poll() (WASM) to execute events whose time has arrived.
     * Events are removed from the scheduled map after execution.
     *
     * @return Number of events processed
     */
    size_t processReadyEvents();

    /**
     * @brief Generate a unique send ID
     *
     * Creates a unique identifier in the format "auto_timestamp_counter"
     * to ensure no collisions with user-provided or other auto-generated IDs.
     *
     * @return Unique send ID string
     */
    std::string generateSendId();

    /**
     * @brief Calculate next wake-up time for timer thread
     *
     * @return Time point of the next event to execute, or max time if no events
     */
    std::chrono::steady_clock::time_point getNextExecutionTime() const;

    /**
     * @brief Calculate next wake-up time (internal use, assumes mutex already locked)
     * CRITICAL FIX: Prevent deadlock by avoiding double locking
     *
     * @return Time point of the next event to execute, or max time if no events
     */
    std::chrono::steady_clock::time_point getNextExecutionTimeUnlocked() const;

#ifndef __EMSCRIPTEN__
    /**
     * @brief Timer thread main loop (Native only)
     *
     * This method runs in a separate thread and processes scheduled events
     * when their execution time arrives. It uses condition variables for
     * efficient waiting and responds to scheduler shutdown requests.
     */
    void timerThreadMain();

    /**
     * @brief Worker thread for asynchronous callback execution (Native only, prevents deadlock)
     */
    void callbackWorker();

    /**
     * @brief Ensure threads are started (Native only, lazy initialization to prevent constructor deadlock)
     */
    void ensureThreadsStarted();
#endif

    // Thread safety - Fine-Grained Locking Strategy
    // PERFORMANCE: Separate mutexes to reduce lock contention and improve concurrent throughput
    mutable std::shared_mutex queueMutex_;  // Protects executionQueue_ and queueSize_
    mutable std::shared_mutex indexMutex_;  // Protects sendIdIndex_ and indexSize_
#ifndef __EMSCRIPTEN__
    // Native: Timer thread coordination
    std::condition_variable_any timerCondition_;  // Timer thread notification
#endif

    // TSAN FIX: Cached next event time to avoid queue access in wait_until predicate
    // This prevents data race when vector reallocation happens during predicate evaluation
    std::chrono::steady_clock::time_point nextEventTime_{std::chrono::steady_clock::time_point::max()};

    // TSAN FIX: Atomic counters to avoid STL container internal pointer races
    std::atomic<size_t> queueSize_{0};
    std::atomic<size_t> indexSize_{0};

    // Event storage: Priority Queue + HashMap hybrid + Per-Session Queues
    // CRITICAL FIX: Use shared_ptr in priority queue for memory safety
    std::priority_queue<std::shared_ptr<ScheduledEvent>, std::vector<std::shared_ptr<ScheduledEvent>>,
                        ExecutionTimeComparator>
        executionQueue_;
    std::unordered_map<std::string, std::shared_ptr<ScheduledEvent>> sendIdIndex_;

    // Per-session sequential execution queues
    // CRITICAL FIX: Use shared_ptr for memory safety consistency
    std::unordered_map<std::string, std::queue<std::shared_ptr<ScheduledEvent>>> sessionQueues_;
    std::unordered_map<std::string, bool> sessionExecuting_;  // Track if session is currently executing

    // === Platform-Specific Execution ===
#ifdef __EMSCRIPTEN__
    // WASM: Polling-based execution (no threads due to QuickJS limitations)
    // QuickJS requires single-threaded access, so we use synchronous polling
    std::atomic<bool> shutdownRequested_{false};
    std::atomic<bool> running_{false};
#else
    // Native: Thread-based execution with timer and callback workers
    // EventScheduler uses separate threads, callback workers serialize JSEngine operations
    std::thread timerThread_;
    std::atomic<bool> shutdownRequested_{false};

    // Callback execution thread pool (prevents deadlock)
    static constexpr size_t CALLBACK_THREAD_POOL_SIZE = 2;
    std::vector<std::thread> callbackThreads_;
    std::queue<std::function<void()>> callbackQueue_;
    std::mutex callbackQueueMutex_;
    std::condition_variable callbackCondition_;
    std::atomic<bool> callbackShutdownRequested_{false};
    std::atomic<bool> running_{false};

    // Thread-local flag to detect if we're running in scheduler's own thread
    // This prevents deadlock when shutdown is called from callback worker thread
    static thread_local bool isInSchedulerThread_;

    // Thread initialization (per-instance to fix static once_flag issue)
    std::once_flag threadsStartedFlag_;
#endif

    // Sequence number for FIFO ordering of events with same executeAt time
    std::atomic<uint64_t> eventSequenceCounter_{0};

    // Event execution
    EventExecutionCallback executionCallback_;

    // W3C SCXML 3.13: Scheduler mode for interactive vs normal execution
    std::atomic<SchedulerMode> mode_{SchedulerMode::AUTOMATIC};

    // W3C SCXML 3.13: Logical time counter for MANUAL mode deterministic stepping
    // In MANUAL mode, events are scheduled and processed based on logical time, not real-time
    // Each forcePoll() advances logical time, enabling deterministic step forward/backward
    //
    // Architecture: Logical time is session-wide, shared by parent-child state machine hierarchy
    // via scheduler sharing (InvokeExecutor.cpp:217-232). This enables consistent MANUAL mode
    // behavior across entire invoke tree without mode propagation logic.
    //
    // Design: Single atomic counter provides single source of truth for logical time progression.
    // Zero duplication ensures deterministic behavior without complex synchronization.
    std::atomic<std::chrono::milliseconds::rep> logicalTime_{0};
};

}  // namespace SCE