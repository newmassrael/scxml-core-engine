#include "events/EventSchedulerImpl.h"
#include "common/Logger.h"
#include "common/UniqueIdGenerator.h"
#include <algorithm>
#include <chrono>
#include <iomanip>
#include <sstream>

namespace SCE {

EventSchedulerImpl::EventSchedulerImpl(EventExecutionCallback executionCallback)
    : executionCallback_(std::move(executionCallback)) {
    if (!executionCallback_) {
        throw std::invalid_argument("EventSchedulerImpl requires a valid execution callback");
    }

    // Initialize running state but DON'T start threads yet to prevent constructor deadlock
    running_ = true;

#ifdef __EMSCRIPTEN__
    // WASM: Polling-based execution (no threads)
    LOG_DEBUG("EventSchedulerImpl: Scheduler started in WASM polling mode");
#else
    // Native: Defer thread creation to prevent deadlock during object construction
    // Threads will be started lazily on first scheduleEvent() call
    LOG_DEBUG("EventSchedulerImpl: Scheduler started with timer thread and {} callback threads",
              CALLBACK_THREAD_POOL_SIZE);
#endif
}

EventSchedulerImpl::~EventSchedulerImpl() {
    // Signal shutdown
    shutdownRequested_ = true;

#ifdef __EMSCRIPTEN__
    // WASM: No threads to clean up
#else
    // Native: Wait for threads (can't detach in destructor context)
    callbackShutdownRequested_ = true;
    callbackCondition_.notify_all();
    timerCondition_.notify_all();

    // Force join callback threads (can't detach in destructor)
    for (auto &thread : callbackThreads_) {
        if (thread.joinable()) {
            thread.join();
        }
    }

    // Force join timer thread (can't detach in destructor)
    if (timerThread_.joinable()) {
        timerThread_.join();
    }
#endif

    // MEMORY LEAK FIX: Explicitly clear all internal data structures
    // This ensures no pending events or hash map entries leak
    {
        std::unique_lock<std::shared_mutex> queueLock(queueMutex_);
        std::unique_lock<std::shared_mutex> indexLock(indexMutex_);

        // Clear priority queue by creating empty queue and swapping
        std::priority_queue<std::shared_ptr<ScheduledEvent>, std::vector<std::shared_ptr<ScheduledEvent>>,
                            ExecutionTimeComparator>
            emptyQueue;
        executionQueue_.swap(emptyQueue);

        // Clear hash maps
        sendIdIndex_.clear();
        sessionQueues_.clear();
        sessionExecuting_.clear();

#ifndef __EMSCRIPTEN__
        // Clear callback queue (Native only)
        std::queue<std::function<void()>> emptyCallbackQueue;
        std::swap(callbackQueue_, emptyCallbackQueue);
#endif

        // Reset atomic counters
        queueSize_.store(0);
        indexSize_.store(0);
    }

    // Now safe to destroy other members
    running_ = false;
}

std::future<std::string> EventSchedulerImpl::scheduleEvent(const EventDescriptor &event,
                                                           std::chrono::milliseconds delay,
                                                           std::shared_ptr<IEventTarget> target,
                                                           const std::string &sendId, const std::string &sessionId) {
    if (!isRunning()) {
        std::promise<std::string> errorPromise;
        errorPromise.set_exception(std::make_exception_ptr(std::runtime_error("EventScheduler is not running")));
        return errorPromise.get_future();
    }

    if (!target) {
        std::promise<std::string> errorPromise;
        errorPromise.set_exception(std::make_exception_ptr(std::invalid_argument("Event target cannot be null")));
        return errorPromise.get_future();
    }

#ifndef __EMSCRIPTEN__
    // Native: Lazy thread initialization to prevent constructor deadlock (before acquiring locks)
    {
        std::unique_lock<std::shared_mutex> queueLock(queueMutex_);
        ensureThreadsStarted();
    }
#endif

    // Generate or use provided send ID
    std::string actualSendId = sendId.empty() ? generateSendId() : sendId;

    // Calculate execution time and sequence number (lock-free operations)
    auto now = std::chrono::steady_clock::now();
    auto executeAt = now + delay;
    uint64_t sequenceNum = eventSequenceCounter_.fetch_add(1, std::memory_order_relaxed);

    // Create scheduled event as shared_ptr for safe async access
    // Store original delay for step backward restoration
    auto scheduledEvent =
        std::make_shared<ScheduledEvent>(event, executeAt, delay, target, actualSendId, sessionId, sequenceNum);

    // W3C SCXML 3.13: Calculate logical execution time for MANUAL mode
    // In MANUAL mode, events are scheduled based on logical time, not real-time
    // This enables deterministic stepping: step backward → forward produces identical results
    if (mode_.load(std::memory_order_acquire) == SchedulerMode::MANUAL) {
        auto currentLogicalTime = std::chrono::milliseconds(logicalTime_.load(std::memory_order_acquire));
        scheduledEvent->logicalExecuteTime = currentLogicalTime + delay;
        LOG_DEBUG("EventSchedulerImpl: Scheduled event '{}' at logical time {}ms (current: {}ms, delay: {}ms)",
                  event.eventName, scheduledEvent->logicalExecuteTime.count(), currentLogicalTime.count(),
                  delay.count());
    }

    auto future = scheduledEvent->sendIdPromise.get_future();

    // Set the send ID promise immediately
    scheduledEvent->sendIdPromise.set_value(actualSendId);

    // PERFORMANCE: Fine-grained locking - acquire index lock first, then queue lock
    // This ordering prevents deadlock (consistent lock ordering across all functions)
    {
        std::unique_lock<std::shared_mutex> indexLock(indexMutex_);

        // Cancel existing event with same send ID (W3C SCXML behavior)
        auto existingIt = sendIdIndex_.find(actualSendId);
        if (existingIt != sendIdIndex_.end()) {
            LOG_DEBUG("EventSchedulerImpl: Cancelling existing event with sendId: {}", actualSendId);
            existingIt->second->cancelled = true;
        }

        // Store in index
        sendIdIndex_[actualSendId] = scheduledEvent;
        indexSize_.fetch_add(1, std::memory_order_release);
    }

    // Now acquire queue lock and add to priority queue
    {
        std::unique_lock<std::shared_mutex> queueLock(queueMutex_);

        executionQueue_.push(scheduledEvent);
        queueSize_.fetch_add(1, std::memory_order_release);

        // Update cached next event time
        if (executeAt < nextEventTime_) {
            nextEventTime_ = executeAt;
        }
    }

    LOG_DEBUG("EventSchedulerImpl: Scheduled event '{}' with sendId '{}' for {}ms delay in session '{}'",
              event.eventName, actualSendId, delay.count(), sessionId);

#ifndef __EMSCRIPTEN__
    // Native: Notify timer thread about new event
    timerCondition_.notify_one();
#endif

    return future;
}

bool EventSchedulerImpl::cancelEvent(const std::string &sendId, const std::string &sessionId) {
    if (sendId.empty()) {
        LOG_WARN("EventSchedulerImpl: Cannot cancel event with empty sendId");
        return false;
    }

    // PERFORMANCE: Immediate cleanup - acquire unique_lock to remove from index
    std::unique_lock<std::shared_mutex> indexLock(indexMutex_);

    auto it = sendIdIndex_.find(sendId);
    if (it != sendIdIndex_.end() && !it->second->cancelled) {
        // W3C SCXML 6.3: Cross-session isolation - events can only be cancelled from the same session
        if (!sessionId.empty() && it->second->sessionId != sessionId) {
            LOG_DEBUG("EventSchedulerImpl: Cross-session cancel blocked - event from '{}', cancel from '{}'",
                      it->second->sessionId, sessionId);
            return false;
        }

        LOG_DEBUG("EventSchedulerImpl: Cancelling event with sendId: {}", sendId);

        // OPTIMIZATION: Mark as cancelled AND immediately remove from index
        it->second->cancelled = true;
        sendIdIndex_.erase(it);
        indexSize_.fetch_sub(1, std::memory_order_release);

#ifndef __EMSCRIPTEN__
        // Native: Notify timer thread about cancellation
        timerCondition_.notify_one();
#endif
        return true;
    }

    LOG_DEBUG("EventSchedulerImpl: Event with sendId '{}' not found or already cancelled (Cross-session cancel attempt "
              "may be blocked)",
              sendId);
    return false;
}

size_t EventSchedulerImpl::cancelEventsForSession(const std::string &sessionId) {
    if (sessionId.empty()) {
        LOG_WARN("EventSchedulerImpl: Cannot cancel events for empty sessionId");
        return 0;
    }

    // PERFORMANCE: Immediate cleanup - acquire unique_lock to remove from index
    std::unique_lock<std::shared_mutex> indexLock(indexMutex_);

    size_t cancelledCount = 0;

    // OPTIMIZATION: Mark as cancelled AND immediately remove from index
    // Use iterator-based erase to avoid invalidating iterators
    for (auto it = sendIdIndex_.begin(); it != sendIdIndex_.end();) {
        if (it->second->sessionId == sessionId && !it->second->cancelled) {
            LOG_DEBUG("EventSchedulerImpl: Cancelling event '{}' with sendId '{}' for session '{}'",
                      it->second->event.eventName, it->first, sessionId);
            it->second->cancelled = true;
            it = sendIdIndex_.erase(it);
            indexSize_.fetch_sub(1, std::memory_order_release);
            cancelledCount++;
        } else {
            ++it;
        }
    }

    if (cancelledCount > 0) {
        LOG_DEBUG("EventSchedulerImpl: Cancelled {} events for session '{}'", cancelledCount, sessionId);
#ifndef __EMSCRIPTEN__
        // Native: Notify timer thread about cancellations
        timerCondition_.notify_one();
#endif
    }

    return cancelledCount;
}

bool EventSchedulerImpl::hasEvent(const std::string &sendId) const {
    if (sendId.empty()) {
        return false;
    }

    // PERFORMANCE: Use shared_lock for read-only find() operation
    std::shared_lock<std::shared_mutex> indexLock(indexMutex_);
    auto it = sendIdIndex_.find(sendId);
    return it != sendIdIndex_.end() && !it->second->cancelled;
}

size_t EventSchedulerImpl::getScheduledEventCount() const {
    // PERFORMANCE: Use shared_lock for read-only size() access
    std::shared_lock<std::shared_mutex> indexLock(indexMutex_);
    return sendIdIndex_.size();
}

void EventSchedulerImpl::shutdown(bool waitForCompletion) {
    // CRITICAL FIX: Always signal shutdown and check threads, even if already marked as not running
    // This ensures destructor can safely join threads that were previously detached
    bool alreadyShutdown = !running_.exchange(false);

    if (!alreadyShutdown) {
        LOG_DEBUG("EventSchedulerImpl: Shutting down scheduler (waitForCompletion={})", waitForCompletion);
    }

    // Always set shutdown flags to signal threads
    shutdownRequested_ = true;

#ifdef __EMSCRIPTEN__
    // WASM: No threads to signal
#else
    // Native: Signal and wait for threads
    callbackShutdownRequested_ = true;

    // Wake up callback threads
    callbackCondition_.notify_all();

    // CRITICAL FIX: Use thread_local flag to detect if we're in scheduler's own thread
    // This is more reliable than comparing thread IDs
    bool calledFromSchedulerThread = isInSchedulerThread_;

    // CRITICAL: Never detach threads - this creates unsafe scenario where threads
    // can outlive the object and access destroyed member variables
    // If called from scheduler thread, skip join to avoid deadlock, but threads
    // will be joined in destructor when called from external context

    // Wait for callback threads to finish
    if (!calledFromSchedulerThread && waitForCompletion) {
        for (auto &thread : callbackThreads_) {
            if (thread.joinable()) {
                thread.join();
            }
        }
    }

    // Wake up timer thread
    timerCondition_.notify_all();

    // Wait for timer thread to finish BEFORE acquiring mutex to prevent deadlock
    if (!calledFromSchedulerThread && waitForCompletion && timerThread_.joinable()) {
        timerThread_.join();
    }
#endif

    // Clear all scheduled events AFTER timer thread has terminated
    // PERFORMANCE: Use fine-grained locking to clear both data structures
    {
        std::unique_lock<std::shared_mutex> indexLock(indexMutex_);
        size_t cancelledCount = sendIdIndex_.size();
        sendIdIndex_.clear();
        indexSize_.store(0, std::memory_order_release);

        if (cancelledCount > 0) {
            LOG_DEBUG("EventSchedulerImpl: Cancelled {} pending events during shutdown", cancelledCount);
        }
    }

    {
        std::unique_lock<std::shared_mutex> queueLock(queueMutex_);
        // Clear the priority queue by creating a new empty one
        std::priority_queue<std::shared_ptr<ScheduledEvent>, std::vector<std::shared_ptr<ScheduledEvent>>,
                            ExecutionTimeComparator>
            emptyQueue;
        executionQueue_.swap(emptyQueue);
        queueSize_.store(0, std::memory_order_release);

        // Clear per-session queues to release all shared_ptr references
        sessionQueues_.clear();
        sessionExecuting_.clear();
    }

#ifndef __EMSCRIPTEN__
    // Native: Clear callback queue to prevent memory leak
    {
        std::unique_lock<std::mutex> callbackLock(callbackQueueMutex_);
        while (!callbackQueue_.empty()) {
            callbackQueue_.pop();
        }
    }
#endif

    LOG_DEBUG("EventSchedulerImpl: Scheduler shutdown complete");
}

bool EventSchedulerImpl::isRunning() const {
    return running_;
}

// === Common: Event processing logic (used by both Native and WASM) ===

void EventSchedulerImpl::executeSessionEventsSync(
    const std::unordered_map<std::string, std::vector<std::shared_ptr<ScheduledEvent>>> &sessionEventGroups,
    const std::string &context) {
    // W3C SCXML 3.13: Synchronous sequential execution within sessions
    // This helper eliminates code duplication between WASM and Native MANUAL mode
    for (auto &[sessionId, sessionEvents] : sessionEventGroups) {
        LOG_DEBUG("EventSchedulerImpl: {} processing {} events for session '{}'", context, sessionEvents.size(),
                  sessionId);

        // Execute events within this session SEQUENTIALLY
        for (auto &eventPtr : sessionEvents) {
            if (!eventPtr) {
                LOG_ERROR("EventSchedulerImpl: NULL shared_ptr in session '{}'", sessionId);
                continue;
            }
            try {
                LOG_DEBUG("EventSchedulerImpl: {} executing event '{}' in session '{}'", context,
                          eventPtr->event.eventName, sessionId);

                // Execute the callback synchronously
                bool success = executionCallback_(eventPtr->event, eventPtr->target, eventPtr->sendId);

                if (success) {
                    LOG_DEBUG("EventSchedulerImpl: Event '{}' executed successfully", eventPtr->event.eventName);
                } else {
                    LOG_WARN("EventSchedulerImpl: Event '{}' execution failed", eventPtr->event.eventName);
                }

            } catch (const std::exception &e) {
                LOG_ERROR("EventSchedulerImpl: Error executing event '{}': {}", eventPtr->event.eventName, e.what());
            }
        }
    }
}

size_t EventSchedulerImpl::processReadyEvents() {
    std::vector<std::shared_ptr<ScheduledEvent>> readyEvents;
    auto now = std::chrono::steady_clock::now();

    // PERFORMANCE: Fine-grained locking - acquire both locks with consistent ordering
    // Lock ordering: index first, then queue (same as scheduleEvent to prevent deadlock)
    std::unique_lock<std::shared_mutex> indexLock(indexMutex_);
    std::unique_lock<std::shared_mutex> queueLock(queueMutex_);

    // Process events from priority queue in execution time order
    while (queueSize_.load(std::memory_order_acquire) > 0) {
        // Copy shared_ptr BEFORE pop() to avoid dangling reference
        std::shared_ptr<ScheduledEvent> topEvent = executionQueue_.top();

        // If event is cancelled, remove from queue only (already removed from index)
        if (topEvent->cancelled) {
            executionQueue_.pop();
            queueSize_.fetch_sub(1, std::memory_order_release);
            LOG_DEBUG("EventSchedulerImpl: Skipping cancelled event from queue: {}", topEvent->sendId);
            continue;
        }

        // W3C SCXML 3.13: Check event readiness based on scheduler mode
        // AUTOMATIC mode: Use real-time (prevents timeout race in normal execution)
        // MANUAL mode: Use logical time (enables deterministic stepping for time-travel debugging)
        if (mode_.load(std::memory_order_acquire) == SchedulerMode::AUTOMATIC) {
            // AUTOMATIC mode: Check if event is ready based on real-time
            if (topEvent->executeAt > now) {
                break;  // Event not ready yet, all remaining events are later
            }
        } else {
            // MANUAL mode: Check if event is ready based on logical time
            // This fixes ForwardBackwardDeterminism: step backward → forward produces identical results
            auto currentLogicalTime = std::chrono::milliseconds(logicalTime_.load(std::memory_order_acquire));
            if (topEvent->logicalExecuteTime > currentLogicalTime) {
                LOG_DEBUG("EventSchedulerImpl: Event '{}' not ready - logical time {}ms < scheduled {}ms",
                          topEvent->event.eventName, currentLogicalTime.count(), topEvent->logicalExecuteTime.count());
                break;  // Event not ready yet in logical time
            }
            LOG_DEBUG("EventSchedulerImpl: Event '{}' ready - logical time {}ms >= scheduled {}ms",
                      topEvent->event.eventName, currentLogicalTime.count(), topEvent->logicalExecuteTime.count());
        }

        // Event is ready - remove from both structures atomically
        executionQueue_.pop();
        queueSize_.fetch_sub(1, std::memory_order_release);

        auto it = sendIdIndex_.find(topEvent->sendId);
        if (it != sendIdIndex_.end()) {
            readyEvents.push_back(it->second);
            sendIdIndex_.erase(it);
            indexSize_.fetch_sub(1, std::memory_order_release);
        } else {
            LOG_WARN("EventSchedulerImpl: Event in queue but not in index - sendId: {}", topEvent->sendId);
        }
    }

    // Release locks before processing events
    queueLock.unlock();
    indexLock.unlock();

    // Process events with per-session sequential execution + inter-session parallelism
    std::unordered_map<std::string, std::vector<std::shared_ptr<ScheduledEvent>>> sessionEventGroups;

    // Group events by session (shared_ptr allows safe copying)
    for (auto &event : readyEvents) {
        sessionEventGroups[event->sessionId].emplace_back(event);
    }

#ifdef __EMSCRIPTEN__
    // WASM: Execute events synchronously on main thread (no callback queue)
    executeSessionEventsSync(sessionEventGroups, "WASM");
#else
    // W3C SCXML 3.13: Native mode execution strategy
    // MANUAL mode: Synchronous execution for deterministic time-travel debugging
    // AUTOMATIC mode: Asynchronous execution for non-blocking performance
    if (mode_.load(std::memory_order_acquire) == SchedulerMode::MANUAL) {
        // MANUAL mode: Execute synchronously for deterministic stepping
        executeSessionEventsSync(sessionEventGroups, "MANUAL mode");
    } else {
        // AUTOMATIC mode: Execute asynchronously via callback queue
        for (auto &[sessionId, sessionEvents] : sessionEventGroups) {
            if (sessionEvents.empty()) {
                continue;
            }

            // Create async task for this session's sequential execution
            auto sessionTask = [this, sessionId, sessionEvents]() {
                LOG_DEBUG("EventSchedulerImpl: Processing {} events for session '{}'", sessionEvents.size(), sessionId);

                // Execute events within this session SEQUENTIALLY
                for (auto &eventPtr : sessionEvents) {
                    if (!eventPtr) {
                        LOG_ERROR("EventSchedulerImpl: NULL shared_ptr in session '{}'", sessionId);
                        continue;
                    }
                    try {
                        LOG_DEBUG("EventSchedulerImpl: Executing event '{}' sequentially in session '{}'",
                                  eventPtr->event.eventName, sessionId);

                        // Execute the callback
                        bool success = executionCallback_(eventPtr->event, eventPtr->target, eventPtr->sendId);

                        if (success) {
                            LOG_DEBUG("EventSchedulerImpl: Event '{}' executed successfully",
                                      eventPtr->event.eventName);
                        } else {
                            LOG_WARN("EventSchedulerImpl: Event '{}' execution failed", eventPtr->event.eventName);
                        }

                    } catch (const std::exception &e) {
                        LOG_ERROR("EventSchedulerImpl: Error executing event '{}': {}", eventPtr->event.eventName,
                                  e.what());
                    }
                }
            };

            // Add to callback queue for asynchronous execution
            {
                std::lock_guard<std::mutex> callbackLock(callbackQueueMutex_);
                callbackQueue_.push(std::move(sessionTask));
            }

            // Notify callback workers
            callbackCondition_.notify_one();
        }
    }
#endif

    return readyEvents.size();
}

#ifndef __EMSCRIPTEN__
// === Native: Thread-based execution methods ===

void EventSchedulerImpl::timerThreadMain() {
    // Mark this thread as a scheduler thread to prevent deadlock on shutdown
    isInSchedulerThread_ = true;

    LOG_DEBUG("EventSchedulerImpl: Timer thread started");

    while (!shutdownRequested_) {
        // PERFORMANCE: Use queue lock for condition variable wait
        std::unique_lock<std::shared_mutex> queueLock(queueMutex_);

        // Use atomic queue size to avoid calling empty()
        size_t currentSize = queueSize_.load(std::memory_order_acquire);

        // Update cached next event time from queue (mutex already held)
        if (currentSize > 0) {
            nextEventTime_ = executionQueue_.top()->executeAt;
        } else {
            nextEventTime_ = std::chrono::steady_clock::time_point::max();
        }
        auto nextExecutionTime = nextEventTime_;

        if (nextExecutionTime == std::chrono::steady_clock::time_point::max()) {
            // No events scheduled, wait indefinitely until notified
            LOG_DEBUG("EventSchedulerImpl: No events scheduled, waiting for notification");
            timerCondition_.wait(
                queueLock, [&] { return shutdownRequested_.load() || queueSize_.load(std::memory_order_acquire) > 0; });
        } else {
            // Wait until next event time or notification
            auto now = std::chrono::steady_clock::now();
            if (nextExecutionTime > now) {
                auto waitTime = std::chrono::duration_cast<std::chrono::milliseconds>(nextExecutionTime - now);
                LOG_DEBUG("EventSchedulerImpl: Waiting {}ms for next event", waitTime.count());

                // Use cached nextEventTime_ instead of accessing queue in predicate
                timerCondition_.wait_until(queueLock, nextExecutionTime, [&] {
                    return shutdownRequested_.load() || nextEventTime_ < nextExecutionTime;
                });
            }
        }

        if (shutdownRequested_) {
            break;
        }

        // Process ready events (releases lock temporarily)
        queueLock.unlock();

        size_t processedCount = processReadyEvents();
        if (processedCount > 0) {
            LOG_DEBUG("EventSchedulerImpl: Processed {} ready events", processedCount);
        }
    }

    LOG_DEBUG("EventSchedulerImpl: Timer thread stopped");
}

void EventSchedulerImpl::ensureThreadsStarted() {
    // Note: This method assumes schedulerMutex_ is already locked by caller

    std::call_once(threadsStartedFlag_, [this]() {
        LOG_DEBUG("EventSchedulerImpl: Starting threads lazily to prevent constructor deadlock");

        // Start callback execution thread pool
        for (size_t i = 0; i < CALLBACK_THREAD_POOL_SIZE; ++i) {
            callbackThreads_.emplace_back(&EventSchedulerImpl::callbackWorker, this);
        }

        // Start timer thread
        timerThread_ = std::thread(&EventSchedulerImpl::timerThreadMain, this);

        LOG_DEBUG("EventSchedulerImpl: All threads started successfully");
    });
}

void EventSchedulerImpl::callbackWorker() {
    // Mark this thread as a scheduler thread to prevent deadlock on shutdown
    isInSchedulerThread_ = true;

    LOG_DEBUG("EventSchedulerImpl: Callback worker thread started");

    while (!callbackShutdownRequested_) {
        std::unique_lock<std::mutex> lock(callbackQueueMutex_);

        // Wait for callback tasks or shutdown
        callbackCondition_.wait(lock, [this] { return !callbackQueue_.empty() || callbackShutdownRequested_.load(); });

        if (callbackShutdownRequested_) {
            break;
        }

        // Get next callback task
        if (!callbackQueue_.empty()) {
            auto task = std::move(callbackQueue_.front());
            callbackQueue_.pop();
            lock.unlock();

            // Execute callback without holding any locks - THIS PREVENTS DEADLOCK
            try {
                task();
            } catch (const std::exception &e) {
                LOG_ERROR("EventSchedulerImpl: Exception in callback worker: {}", e.what());
            } catch (...) {
                LOG_ERROR("EventSchedulerImpl: Unknown exception in callback worker");
            }
        }
    }

    LOG_DEBUG("EventSchedulerImpl: Callback worker thread stopped");
}

// Thread-local variable definition
thread_local bool EventSchedulerImpl::isInSchedulerThread_ = false;

#endif  // __EMSCRIPTEN__

// === Polling method (Common for both Native and WASM) ===

#ifdef __EMSCRIPTEN__
size_t EventSchedulerImpl::poll() {
    if (!isRunning()) {
        return 0;
    }

    // W3C SCXML 6.2: Process all ready events synchronously (WASM only)
    // WASM builds have no timer thread, so poll() must be called manually
    return processReadyEvents();
}
#endif

std::string EventSchedulerImpl::generateSendId() {
    // REFACTOR: Use centralized UniqueIdGenerator instead of duplicate logic
    return UniqueIdGenerator::generateSendId();
}

std::chrono::steady_clock::time_point EventSchedulerImpl::getNextExecutionTime() const {
    // PERFORMANCE: Use shared_lock for read-only access to queue
    std::shared_lock<std::shared_mutex> queueLock(queueMutex_);
    return getNextExecutionTimeUnlocked();
}

std::chrono::steady_clock::time_point EventSchedulerImpl::getNextExecutionTimeUnlocked() const {
    // Internal method assumes queueMutex_ is already locked by caller
    if (executionQueue_.empty()) {
        return std::chrono::steady_clock::time_point::max();
    }

    // Peek at the top event without removing it
    const auto &topEvent = executionQueue_.top();

    // If the top event is cancelled, we still return its time
    // This is safe because processReadyEvents() will skip cancelled events
    return topEvent->executeAt;
}

std::vector<ScheduledEventInfo> EventSchedulerImpl::getScheduledEvents() const {
    std::vector<ScheduledEventInfo> result;
    auto now = std::chrono::steady_clock::now();

    // PERFORMANCE: Use shared_lock for read-only access to index
    std::shared_lock<std::shared_mutex> indexLock(indexMutex_);

    // Iterate through sendIdIndex_ to get all scheduled events
    // Use index instead of queue to avoid copying priority_queue
    for (const auto &[sendId, event] : sendIdIndex_) {
        // Skip cancelled events
        if (event->cancelled) {
            continue;
        }

        // Calculate remaining time until execution
        auto remaining = std::chrono::duration_cast<std::chrono::milliseconds>(event->executeAt - now);

        // Add to result with complete EventDescriptor fields for step backward restoration
        result.push_back({event->event.eventName, event->sendId, remaining, event->originalDelay, event->sessionId,
                          event->event.target, event->event.type, event->event.data, event->event.content,
                          event->event.params});
    }

    // Sort by remaining time (earliest first)
    std::sort(result.begin(), result.end(), [](const ScheduledEventInfo &a, const ScheduledEventInfo &b) {
        return a.remainingTime < b.remainingTime;
    });

    return result;
}

void EventSchedulerImpl::setMode(SchedulerMode mode) {
    mode_.store(mode, std::memory_order_release);
    LOG_INFO("EventSchedulerImpl: Scheduler mode set to {}", mode == SchedulerMode::AUTOMATIC ? "AUTOMATIC" : "MANUAL");
}

SchedulerMode EventSchedulerImpl::getMode() const {
    return mode_.load(std::memory_order_acquire);
}

size_t EventSchedulerImpl::forcePoll() {
    if (!isRunning()) {
        return 0;
    }

    // W3C SCXML 3.13: In MANUAL mode, advance logical time to next scheduled event
    // This enables deterministic stepping: each forcePoll() advances to the next event's logical time
    // In AUTOMATIC mode, this is a no-op (logical time is unused)
    //
    // Design Pattern: "Jump to Event" instead of "Increment by Fixed Step"
    // - Advantages: Handles variable delays naturally, skips empty time periods
    // - Trade-offs: Cannot step "between" events, always jumps to next event boundary
    // - Determinism: Same event sequence produces same logical time progression
    if (mode_.load(std::memory_order_acquire) == SchedulerMode::MANUAL) {
        std::shared_lock<std::shared_mutex> queueLock(queueMutex_);

        if (!executionQueue_.empty()) {
            // Advance logical time to the next scheduled event's logical time
            // This allows processReadyEvents() to process all events at this logical time
            auto nextEvent = executionQueue_.top();
            auto newLogicalTime = nextEvent->logicalExecuteTime.count();
            auto oldLogicalTime = logicalTime_.exchange(newLogicalTime, std::memory_order_release);

            LOG_DEBUG("EventSchedulerImpl: MANUAL mode - advanced logical time from {}ms to {}ms (next event: '{}')",
                      oldLogicalTime, newLogicalTime, nextEvent->event.eventName);
        } else {
            LOG_DEBUG("EventSchedulerImpl: MANUAL mode - no scheduled events, logical time unchanged at {}ms",
                      logicalTime_.load(std::memory_order_acquire));
        }
    }

    LOG_DEBUG("EventSchedulerImpl: forcePoll() called - processing ready events");
    return processReadyEvents();
}

std::chrono::milliseconds EventSchedulerImpl::getLogicalTime() const {
    return std::chrono::milliseconds(logicalTime_.load(std::memory_order_acquire));
}

void EventSchedulerImpl::setLogicalTime(std::chrono::milliseconds timeMs) {
    auto oldTime = logicalTime_.exchange(timeMs.count(), std::memory_order_release);
    LOG_DEBUG("EventSchedulerImpl: Logical time set from {}ms to {}ms (snapshot restoration)", oldTime, timeMs.count());
}

}  // namespace SCE
