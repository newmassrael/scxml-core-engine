// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "events/EventSchedulerImpl.h"
#include "core/LogMacros.h"
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

    running_ = true;

#ifdef __EMSCRIPTEN__
    SCE_LOG_DEBUG("EventSchedulerImpl: Scheduler started in WASM polling mode");
#else
    SCE_LOG_DEBUG("EventSchedulerImpl: Scheduler started with timer thread and {} callback threads",
              CALLBACK_THREAD_POOL_SIZE);
#endif
}

EventSchedulerImpl::~EventSchedulerImpl() {
    shutdownRequested_ = true;

#ifdef __EMSCRIPTEN__
    // WASM: No threads to clean up
#else
    callbackShutdownRequested_ = true;
    callbackCondition_.notify_all();
    timerCondition_.notify_all();

    for (auto &thread : callbackThreads_) {
        if (thread.joinable()) {
            thread.join();
        }
    }

    if (timerThread_.joinable()) {
        timerThread_.join();
    }
#endif

    {
        std::unique_lock<std::shared_mutex> lock(mutex_);

        // Clear std::map (simpler than priority_queue)
        executionQueue_.clear();
        sendIdIndex_.clear();
        sessionQueues_.clear();
        sessionExecuting_.clear();

#ifndef __EMSCRIPTEN__
        std::queue<std::function<void()>> emptyCallbackQueue;
        std::swap(callbackQueue_, emptyCallbackQueue);
#endif

        queueSize_.store(0);
    }

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
    {
        std::unique_lock<std::shared_mutex> lock(mutex_);
        ensureThreadsStarted();
    }
#endif

    std::string actualSendId = sendId.empty() ? generateSendId() : sendId;

    auto now = std::chrono::steady_clock::now();
    auto executeAt = now + delay;
    uint64_t sequenceNum = eventSequenceCounter_.fetch_add(1, std::memory_order_relaxed);

    auto scheduledEvent =
        std::make_shared<ScheduledEvent>(event, executeAt, delay, target, actualSendId, sessionId, sequenceNum);

    if (mode_.load(std::memory_order_acquire) == SchedulerMode::MANUAL) {
        auto currentLogicalTime = std::chrono::milliseconds(logicalTime_.load(std::memory_order_acquire));
        scheduledEvent->logicalExecuteTime = currentLogicalTime + delay;
        SCE_LOG_DEBUG("EventSchedulerImpl: Scheduled event '{}' at logical time {}ms (current: {}ms, delay: {}ms)",
                  event.eventName, scheduledEvent->logicalExecuteTime.count(), currentLogicalTime.count(),
                  delay.count());
    }

    auto future = scheduledEvent->sendIdPromise.get_future();
    scheduledEvent->sendIdPromise.set_value(actualSendId);

    // §scxml-6.3: ACTUAL removal of existing event (Zero Duplication pattern)
    {
        std::unique_lock<std::shared_mutex> lock(mutex_);

        // Cancel existing event with same sendId - ACTUAL removal, not lazy marking!
        auto existingIt = sendIdIndex_.find(actualSendId);
        if (existingIt != sendIdIndex_.end()) {
            SCE_LOG_DEBUG("EventSchedulerImpl: Cancelling existing event with sendId: {}", actualSendId);
            executionQueue_.erase(existingIt->second);  // ACTUAL removal from queue!
            sendIdIndex_.erase(existingIt);
            queueSize_.fetch_sub(1, std::memory_order_release);
        }

        // Insert new event into ordered map
        OrderKey key{executeAt, sequenceNum};
        auto insertResult = executionQueue_.emplace(key, scheduledEvent);

        // Store iterator for O(1) cancel lookup
        sendIdIndex_[actualSendId] = insertResult.first;
        queueSize_.fetch_add(1, std::memory_order_release);

        // Update cached next event time
        if (executeAt < nextEventTime_) {
            nextEventTime_ = executeAt;
        }
    }

    SCE_LOG_DEBUG("EventSchedulerImpl: Scheduled event '{}' with sendId '{}' for {}ms delay in session '{}'",
              event.eventName, actualSendId, delay.count(), sessionId);

#ifndef __EMSCRIPTEN__
    timerCondition_.notify_one();
#endif

    return future;
}

bool EventSchedulerImpl::cancelEvent(const std::string &sendId, const std::string &sessionId) {
    if (sendId.empty()) {
        SCE_LOG_WARN("EventSchedulerImpl: Cannot cancel event with empty sendId");
        return false;
    }

    std::unique_lock<std::shared_mutex> lock(mutex_);

    auto it = sendIdIndex_.find(sendId);
    if (it != sendIdIndex_.end()) {
        // §scxml-6.3: Cross-session isolation
        if (!sessionId.empty() && it->second->second->sessionId != sessionId) {
            SCE_LOG_DEBUG("EventSchedulerImpl: Cross-session cancel blocked - event from '{}', cancel from '{}'",
                      it->second->second->sessionId, sessionId);
            return false;
        }

        SCE_LOG_DEBUG("EventSchedulerImpl: Cancelling event with sendId: {}", sendId);

        // §scxml-6.3: ACTUAL removal (Zero Duplication pattern)
        executionQueue_.erase(it->second);  // ACTUAL removal from queue!
        sendIdIndex_.erase(it);
        queueSize_.fetch_sub(1, std::memory_order_release);

#ifndef __EMSCRIPTEN__
        timerCondition_.notify_one();
#endif
        return true;
    }

    SCE_LOG_DEBUG("EventSchedulerImpl: Event with sendId '{}' not found", sendId);
    return false;
}

size_t EventSchedulerImpl::cancelEventsForSession(const std::string &sessionId) {
    if (sessionId.empty()) {
        SCE_LOG_WARN("EventSchedulerImpl: Cannot cancel events for empty sessionId");
        return 0;
    }

    std::unique_lock<std::shared_mutex> lock(mutex_);

    size_t cancelledCount = 0;

    // Collect sendIds to cancel (can't modify while iterating)
    std::vector<std::string> sendIdsToCancel;
    for (const auto &[sendId, queueIt] : sendIdIndex_) {
        if (queueIt->second->sessionId == sessionId) {
            sendIdsToCancel.push_back(sendId);
        }
    }

    // §scxml-6.3: ACTUAL removal for each event
    for (const auto &sendId : sendIdsToCancel) {
        auto indexIt = sendIdIndex_.find(sendId);
        if (indexIt != sendIdIndex_.end()) {
            SCE_LOG_DEBUG("EventSchedulerImpl: Cancelling event with sendId '{}' for session '{}'", sendId, sessionId);
            executionQueue_.erase(indexIt->second);  // ACTUAL removal!
            sendIdIndex_.erase(indexIt);
            queueSize_.fetch_sub(1, std::memory_order_release);
            cancelledCount++;
        }
    }

    if (cancelledCount > 0) {
        SCE_LOG_DEBUG("EventSchedulerImpl: Cancelled {} events for session '{}'", cancelledCount, sessionId);
#ifndef __EMSCRIPTEN__
        timerCondition_.notify_one();
#endif
    }

    return cancelledCount;
}

bool EventSchedulerImpl::hasEvent(const std::string &sendId) const {
    if (sendId.empty()) {
        return false;
    }

    std::shared_lock<std::shared_mutex> lock(mutex_);
    return sendIdIndex_.find(sendId) != sendIdIndex_.end();
}

size_t EventSchedulerImpl::getScheduledEventCount() const {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    return sendIdIndex_.size();
}

void EventSchedulerImpl::shutdown(bool waitForCompletion) {
    bool alreadyShutdown = !running_.exchange(false);

    if (!alreadyShutdown) {
        SCE_LOG_DEBUG("EventSchedulerImpl: Shutting down scheduler (waitForCompletion={})", waitForCompletion);
    }

    shutdownRequested_ = true;

#ifdef __EMSCRIPTEN__
    // WASM: No threads to signal
#else
    callbackShutdownRequested_ = true;
    callbackCondition_.notify_all();

    bool calledFromSchedulerThread = isInSchedulerThread_;

    if (!calledFromSchedulerThread && waitForCompletion) {
        for (auto &thread : callbackThreads_) {
            if (thread.joinable()) {
                thread.join();
            }
        }
    }

    timerCondition_.notify_all();

    if (!calledFromSchedulerThread && waitForCompletion && timerThread_.joinable()) {
        timerThread_.join();
    }
#endif

    {
        std::unique_lock<std::shared_mutex> lock(mutex_);
        size_t cancelledCount = sendIdIndex_.size();
        sendIdIndex_.clear();
        executionQueue_.clear();
        queueSize_.store(0, std::memory_order_release);
        sessionQueues_.clear();
        sessionExecuting_.clear();

        if (cancelledCount > 0) {
            SCE_LOG_DEBUG("EventSchedulerImpl: Cancelled {} pending events during shutdown", cancelledCount);
        }
    }

#ifndef __EMSCRIPTEN__
    {
        std::unique_lock<std::mutex> callbackLock(callbackQueueMutex_);
        while (!callbackQueue_.empty()) {
            callbackQueue_.pop();
        }
    }
#endif

    SCE_LOG_DEBUG("EventSchedulerImpl: Scheduler shutdown complete");
}

bool EventSchedulerImpl::isRunning() const {
    return running_;
}

void EventSchedulerImpl::executeSessionEventsSync(
    const std::unordered_map<std::string, std::vector<ScheduledEventPtr>> &sessionEventGroups,
    const std::string &context) {
    for (auto &[sessionId, sessionEvents] : sessionEventGroups) {
        SCE_LOG_DEBUG("EventSchedulerImpl: {} processing {} events for session '{}'", context, sessionEvents.size(),
                  sessionId);

        for (auto &eventPtr : sessionEvents) {
            if (!eventPtr) {
                SCE_LOG_ERROR("EventSchedulerImpl: NULL shared_ptr in session '{}'", sessionId);
                continue;
            }
            try {
                SCE_LOG_DEBUG("EventSchedulerImpl: {} executing event '{}' in session '{}' at logical time {}ms", context,
                          eventPtr->event.eventName, sessionId, eventPtr->logicalExecuteTime.count());

                EventDescriptor eventWithTimestamp = std::move(eventPtr->event);
                eventWithTimestamp.logicalExecuteTime = eventPtr->logicalExecuteTime;

                bool success = executionCallback_(eventWithTimestamp, eventPtr->target, eventPtr->sendId);

                if (success) {
                    SCE_LOG_DEBUG("EventSchedulerImpl: Event '{}' executed successfully", eventPtr->event.eventName);
                } else {
                    SCE_LOG_WARN("EventSchedulerImpl: Event '{}' execution failed", eventPtr->event.eventName);
                }

            } catch (const std::exception &e) {
                SCE_LOG_ERROR("EventSchedulerImpl: Error executing event '{}': {}", eventPtr->event.eventName, e.what());
            }
        }
    }
}

size_t EventSchedulerImpl::processReadyEvents() {
    std::vector<ScheduledEventPtr> readyEvents;
    auto now = std::chrono::steady_clock::now();

    std::unique_lock<std::shared_mutex> lock(mutex_);

    // Process events from ordered map (no cancelled flag check needed - actual deletion!)
    while (!executionQueue_.empty()) {
        auto it = executionQueue_.begin();
        auto &scheduledEvent = it->second;

        // §scxml-6.2.3: Check delayed-event readiness (dispatch only when delay interval elapses)
        if (mode_.load(std::memory_order_acquire) == SchedulerMode::AUTOMATIC) {
            if (it->first.executeAt > now) {
                break;  // Event not ready yet
            }
        } else {
            auto currentLogicalTime = std::chrono::milliseconds(logicalTime_.load(std::memory_order_acquire));
            if (scheduledEvent->logicalExecuteTime > currentLogicalTime) {
                SCE_LOG_DEBUG("EventSchedulerImpl: Event '{}' not ready - logical time {}ms < scheduled {}ms",
                          scheduledEvent->event.eventName, currentLogicalTime.count(),
                          scheduledEvent->logicalExecuteTime.count());
                break;
            }
            SCE_LOG_DEBUG("EventSchedulerImpl: Event '{}' ready - logical time {}ms >= scheduled {}ms",
                      scheduledEvent->event.eventName, currentLogicalTime.count(),
                      scheduledEvent->logicalExecuteTime.count());
        }

        // Event is ready - collect and remove
        readyEvents.push_back(scheduledEvent);
        sendIdIndex_.erase(scheduledEvent->sendId);
        executionQueue_.erase(it);
        queueSize_.fetch_sub(1, std::memory_order_release);
    }

    lock.unlock();

    // Group events by session
    std::unordered_map<std::string, std::vector<ScheduledEventPtr>> sessionEventGroups;
    for (auto &event : readyEvents) {
        sessionEventGroups[event->sessionId].emplace_back(event);
    }

#ifdef __EMSCRIPTEN__
    executeSessionEventsSync(sessionEventGroups, "WASM");
#else
    if (mode_.load(std::memory_order_acquire) == SchedulerMode::MANUAL) {
        executeSessionEventsSync(sessionEventGroups, "MANUAL mode");
    } else {
        for (auto &[sessionId, sessionEvents] : sessionEventGroups) {
            if (sessionEvents.empty()) {
                continue;
            }

            auto sessionTask = [this, sessionId, sessionEvents]() {
                SCE_LOG_DEBUG("EventSchedulerImpl: Processing {} events for session '{}'", sessionEvents.size(), sessionId);

                for (auto &eventPtr : sessionEvents) {
                    if (!eventPtr) {
                        SCE_LOG_ERROR("EventSchedulerImpl: NULL shared_ptr in session '{}'", sessionId);
                        continue;
                    }
                    try {
                        SCE_LOG_DEBUG("EventSchedulerImpl: Executing event '{}' sequentially in session '{}' at logical "
                                  "time {}ms",
                                  eventPtr->event.eventName, sessionId, eventPtr->logicalExecuteTime.count());

                        EventDescriptor eventWithTimestamp = std::move(eventPtr->event);
                        eventWithTimestamp.logicalExecuteTime = eventPtr->logicalExecuteTime;

                        bool success = executionCallback_(eventWithTimestamp, eventPtr->target, eventPtr->sendId);

                        if (success) {
                            SCE_LOG_DEBUG("EventSchedulerImpl: Event '{}' executed successfully",
                                      eventPtr->event.eventName);
                        } else {
                            SCE_LOG_WARN("EventSchedulerImpl: Event '{}' execution failed", eventPtr->event.eventName);
                        }

                    } catch (const std::exception &e) {
                        SCE_LOG_ERROR("EventSchedulerImpl: Error executing event '{}': {}", eventPtr->event.eventName,
                                  e.what());
                    }
                }
            };

            {
                std::lock_guard<std::mutex> callbackLock(callbackQueueMutex_);
                callbackQueue_.push(std::move(sessionTask));
            }

            callbackCondition_.notify_one();
        }
    }
#endif

    return readyEvents.size();
}

#ifndef __EMSCRIPTEN__

void EventSchedulerImpl::timerThreadMain() {
    isInSchedulerThread_ = true;

    SCE_LOG_DEBUG("EventSchedulerImpl: Timer thread started");

    while (!shutdownRequested_) {
        std::unique_lock<std::shared_mutex> lock(mutex_);

        // Update cached next event time from ordered map
        if (!executionQueue_.empty()) {
            nextEventTime_ = executionQueue_.begin()->first.executeAt;
        } else {
            nextEventTime_ = std::chrono::steady_clock::time_point::max();
        }
        auto nextExecutionTime = nextEventTime_;

        if (nextExecutionTime == std::chrono::steady_clock::time_point::max()) {
            SCE_LOG_DEBUG("EventSchedulerImpl: No events scheduled, waiting for notification");
            timerCondition_.wait(lock, [&] { return shutdownRequested_.load() || !executionQueue_.empty(); });
        } else {
            auto now = std::chrono::steady_clock::now();
            if (nextExecutionTime > now) {
                auto waitTime = std::chrono::duration_cast<std::chrono::milliseconds>(nextExecutionTime - now);
                SCE_LOG_DEBUG("EventSchedulerImpl: Waiting {}ms for next event", waitTime.count());

                timerCondition_.wait_until(lock, nextExecutionTime, [&] {
                    return shutdownRequested_.load() || nextEventTime_ < nextExecutionTime;
                });
            }
        }

        if (shutdownRequested_) {
            break;
        }

        lock.unlock();

        size_t processedCount = processReadyEvents();
        if (processedCount > 0) {
            SCE_LOG_DEBUG("EventSchedulerImpl: Processed {} ready events", processedCount);
        }
    }

    SCE_LOG_DEBUG("EventSchedulerImpl: Timer thread stopped");
}

void EventSchedulerImpl::ensureThreadsStarted() {
    std::call_once(threadsStartedFlag_, [this]() {
        SCE_LOG_DEBUG("EventSchedulerImpl: Starting threads lazily to prevent constructor deadlock");

        for (size_t i = 0; i < CALLBACK_THREAD_POOL_SIZE; ++i) {
            callbackThreads_.emplace_back(&EventSchedulerImpl::callbackWorker, this);
        }

        timerThread_ = std::thread(&EventSchedulerImpl::timerThreadMain, this);

        SCE_LOG_DEBUG("EventSchedulerImpl: All threads started successfully");
    });
}

void EventSchedulerImpl::callbackWorker() {
    isInSchedulerThread_ = true;

    SCE_LOG_DEBUG("EventSchedulerImpl: Callback worker thread started");

    while (!callbackShutdownRequested_) {
        std::unique_lock<std::mutex> lock(callbackQueueMutex_);

        callbackCondition_.wait(lock, [this] { return !callbackQueue_.empty() || callbackShutdownRequested_.load(); });

        if (callbackShutdownRequested_) {
            break;
        }

        if (!callbackQueue_.empty()) {
            auto task = std::move(callbackQueue_.front());
            callbackQueue_.pop();
            lock.unlock();

            try {
                task();
            } catch (const std::exception &e) {
                SCE_LOG_ERROR("EventSchedulerImpl: Exception in callback worker: {}", e.what());
            } catch (...) {
                SCE_LOG_ERROR("EventSchedulerImpl: Unknown exception in callback worker");
            }
        }
    }

    SCE_LOG_DEBUG("EventSchedulerImpl: Callback worker thread stopped");
}

thread_local bool EventSchedulerImpl::isInSchedulerThread_ = false;

#endif  // __EMSCRIPTEN__

#ifdef __EMSCRIPTEN__
size_t EventSchedulerImpl::poll() {
    if (!isRunning()) {
        return 0;
    }

    return processReadyEvents();
}
#endif

std::string EventSchedulerImpl::generateSendId() {
    return UniqueIdGenerator::generateSendId();
}

std::chrono::steady_clock::time_point EventSchedulerImpl::getNextExecutionTime() const {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    return getNextExecutionTimeUnlocked();
}

std::chrono::steady_clock::time_point EventSchedulerImpl::getNextExecutionTimeUnlocked() const {
    if (executionQueue_.empty()) {
        return std::chrono::steady_clock::time_point::max();
    }
    return executionQueue_.begin()->first.executeAt;
}

std::vector<ScheduledEventInfo> EventSchedulerImpl::getScheduledEvents() const {
    std::vector<ScheduledEventInfo> result;
    auto now = std::chrono::steady_clock::now();

    std::shared_lock<std::shared_mutex> lock(mutex_);

    // Iterate through ordered map (already sorted by time)
    for (const auto &[key, event] : executionQueue_) {
        auto remaining = std::chrono::duration_cast<std::chrono::milliseconds>(event->executeAt - now);

        result.push_back({event->event.eventName, event->sendId, remaining, event->originalDelay, event->sessionId,
                          event->event.target, event->event.type, event->event.data, event->event.content,
                          event->event.params});
    }

    return result;
}

void EventSchedulerImpl::setMode(SchedulerMode mode) {
    mode_.store(mode, std::memory_order_release);
    SCE_LOG_INFO("EventSchedulerImpl: Scheduler mode set to {}", mode == SchedulerMode::AUTOMATIC ? "AUTOMATIC" : "MANUAL");
}

SchedulerMode EventSchedulerImpl::getMode() const {
    return mode_.load(std::memory_order_acquire);
}

size_t EventSchedulerImpl::forcePoll() {
    if (!isRunning()) {
        return 0;
    }

    if (mode_.load(std::memory_order_acquire) == SchedulerMode::MANUAL) {
        std::shared_lock<std::shared_mutex> lock(mutex_);

        if (!executionQueue_.empty()) {
            auto nextEvent = executionQueue_.begin()->second;
            auto newLogicalTime = nextEvent->logicalExecuteTime.count();
            auto oldLogicalTime = logicalTime_.exchange(newLogicalTime, std::memory_order_release);

            SCE_LOG_DEBUG("EventSchedulerImpl: MANUAL mode - advanced logical time from {}ms to {}ms (next event: '{}')",
                      oldLogicalTime, newLogicalTime, nextEvent->event.eventName);
        } else {
            SCE_LOG_DEBUG("EventSchedulerImpl: MANUAL mode - no scheduled events, logical time unchanged at {}ms",
                      logicalTime_.load(std::memory_order_acquire));
        }
    }

    SCE_LOG_DEBUG("EventSchedulerImpl: forcePoll() called - processing ready events");
    return processReadyEvents();
}

std::chrono::milliseconds EventSchedulerImpl::getLogicalTime() const {
    return std::chrono::milliseconds(logicalTime_.load(std::memory_order_acquire));
}

void EventSchedulerImpl::setLogicalTime(std::chrono::milliseconds timeMs) {
    auto oldTime = logicalTime_.exchange(timeMs.count(), std::memory_order_release);
    SCE_LOG_DEBUG("EventSchedulerImpl: Logical time set from {}ms to {}ms (snapshot restoration)", oldTime, timeMs.count());
}

}  // namespace SCE
