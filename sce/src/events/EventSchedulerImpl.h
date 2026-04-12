// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

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
#include <shared_mutex>
#include <string>
#include <thread>
#include <unordered_map>
#include <vector>

namespace SCE {

/**
 * @brief Concrete implementation of IEventScheduler (Push-based, thread-safe)
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
 * - W3C SCXML compliant behavior: ACTUAL event removal on cancel (not lazy)
 *
 * Design: Uses std::map instead of priority_queue for O(log n) actual deletion
 *
 * Zero Duplication Strategy:
 * - SAME ALGORITHM as SchedulerQueueCore: std::map<OrderKey, Entry> + sendIdIndex_
 * - SEPARATE IMPLEMENTATION due to complexity difference:
 *   - Thread-safety (mutex, condition_variable)
 *   - Session management (sessionQueues_, sessionExecuting_)
 *   - Async promise handling (sendIdPromise)
 *   - MANUAL mode logical time support
 *   - Callback thread pool
 * - PullScheduler: Lightweight wrapper around SchedulerQueueCore (single-threaded)
 * - EventSchedulerImpl: Full-featured push-based scheduler (multi-threaded)
 */
class EventSchedulerImpl : public IEventScheduler {
public:
    explicit EventSchedulerImpl(EventExecutionCallback executionCallback);
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

    std::chrono::milliseconds getLogicalTime() const;
    void setLogicalTime(std::chrono::milliseconds timeMs);

#ifdef __EMSCRIPTEN__
    size_t poll();
#endif

private:
    /**
     * @brief Internal structure for scheduled events
     *
     * Note: No 'cancelled' flag - W3C SCXML 6.3 compliance via ACTUAL deletion
     */
    struct ScheduledEvent {
        EventDescriptor event;
        std::chrono::steady_clock::time_point executeAt;
        std::chrono::milliseconds originalDelay;
        std::shared_ptr<IEventTarget> target;
        std::promise<std::string> sendIdPromise;
        std::string sendId;
        std::string sessionId;
        uint64_t sequenceNumber = 0;
        std::chrono::milliseconds logicalExecuteTime{0};

        ScheduledEvent(const EventDescriptor &evt, std::chrono::steady_clock::time_point execTime,
                       std::chrono::milliseconds origDelay, std::shared_ptr<IEventTarget> tgt, const std::string &id,
                       const std::string &sessId, uint64_t seqNum)
            : event(evt), executeAt(execTime), originalDelay(origDelay), target(std::move(tgt)), sendId(id),
              sessionId(sessId), sequenceNumber(seqNum) {}
    };

    /**
     * @brief Key for std::map ordering (executeAt, sequenceNumber)
     *
     * Zero Duplication: Same pattern as SchedulerQueueCore::OrderKey
     */
    struct OrderKey {
        std::chrono::steady_clock::time_point executeAt;
        uint64_t sequenceNumber;

        bool operator<(const OrderKey &other) const {
            if (executeAt != other.executeAt) {
                return executeAt < other.executeAt;
            }
            return sequenceNumber < other.sequenceNumber;
        }
    };

    using ScheduledEventPtr = std::shared_ptr<ScheduledEvent>;
    using ExecutionQueueType = std::map<OrderKey, ScheduledEventPtr>;
    using QueueIterator = ExecutionQueueType::iterator;

    size_t processReadyEvents();
    std::string generateSendId();
    std::chrono::steady_clock::time_point getNextExecutionTime() const;
    std::chrono::steady_clock::time_point getNextExecutionTimeUnlocked() const;

    void
    executeSessionEventsSync(const std::unordered_map<std::string, std::vector<ScheduledEventPtr>> &sessionEventGroups,
                             const std::string &context);

#ifndef __EMSCRIPTEN__
    void timerThreadMain();
    void callbackWorker();
    void ensureThreadsStarted();
#endif

    // Thread safety: Single mutex for both queue and index (always modified together)
    mutable std::shared_mutex mutex_;
#ifndef __EMSCRIPTEN__
    std::condition_variable_any timerCondition_;
#endif

    std::chrono::steady_clock::time_point nextEventTime_{std::chrono::steady_clock::time_point::max()};
    std::atomic<size_t> queueSize_{0};

    // Event storage: std::map for O(log n) actual deletion (W3C SCXML 6.3 compliance)
    // Zero Duplication: Same pattern as SchedulerQueueCore
    ExecutionQueueType executionQueue_;
    std::unordered_map<std::string, QueueIterator> sendIdIndex_;

    // Per-session sequential execution queues
    std::unordered_map<std::string, std::queue<ScheduledEventPtr>> sessionQueues_;
    std::unordered_map<std::string, bool> sessionExecuting_;

#ifdef __EMSCRIPTEN__
    std::atomic<bool> shutdownRequested_{false};
    std::atomic<bool> running_{false};
#else
    std::thread timerThread_;
    std::atomic<bool> shutdownRequested_{false};

    static constexpr size_t CALLBACK_THREAD_POOL_SIZE = 2;
    std::vector<std::thread> callbackThreads_;
    std::queue<std::function<void()>> callbackQueue_;
    std::mutex callbackQueueMutex_;
    std::condition_variable callbackCondition_;
    std::atomic<bool> callbackShutdownRequested_{false};
    std::atomic<bool> running_{false};

    static thread_local bool isInSchedulerThread_;
    std::once_flag threadsStartedFlag_;
#endif

    std::atomic<uint64_t> eventSequenceCounter_{0};
    EventExecutionCallback executionCallback_;
    std::atomic<SchedulerMode> mode_{SchedulerMode::AUTOMATIC};
    std::atomic<std::chrono::milliseconds::rep> logicalTime_{0};
};

}  // namespace SCE
