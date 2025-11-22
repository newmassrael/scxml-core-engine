#include "runtime/EventRaiserImpl.h"
#include "common/EventTypeHelper.h"
#include "common/Logger.h"
#include "common/StringUtils.h"
#include "events/IEventDispatcher.h"
#include "events/PlatformEventRaiserHelper.h"
#include "runtime/StateSnapshot.h"
#include <mutex>

namespace SCE {

// W3C SCXML 6.4: Thread-local storage for origin session ID during event callback execution
thread_local std::string EventRaiserImpl::currentOriginSessionId_;

// W3C SCXML 5.10: Thread-local storage for send ID from failed send elements (for error events)
thread_local std::string EventRaiserImpl::currentSendId_;

// W3C SCXML 5.10: Thread-local storage for invoke ID from invoked child processes (test 338)
thread_local std::string EventRaiserImpl::currentInvokeId_;

// W3C SCXML 5.10: Thread-local storage for origin type from event processor (test 253, 331, 352, 372)
thread_local std::string EventRaiserImpl::currentOriginType_;

// W3C SCXML 5.10: Thread-local storage for event type ("internal", "platform", "external") (test 331)
thread_local std::string EventRaiserImpl::currentEventType_;

EventRaiserImpl::EventRaiserImpl(EventCallback callback)
    : eventCallback_(std::move(callback)), scheduler_(nullptr), shutdownRequested_(false), isRunning_(false),
      immediateMode_(false) {
    LOG_DEBUG("EventRaiserImpl: Created with callback: {} (instance: {})", (eventCallback_ ? "set" : "none"),
              (void *)this);

    // Zero Duplication Principle: Platform-specific initialization through Helper
    // Note: scheduler_ will be set later via setScheduler() for delayed event polling support
    platformHelper_ = createPlatformEventRaiserHelper(this, scheduler_);
    platformHelper_->start();

    LOG_DEBUG("EventRaiserImpl: Platform-specific initialization complete");
}

EventRaiserImpl::~EventRaiserImpl() {
    shutdown();
}

void EventRaiserImpl::setScheduler(std::shared_ptr<IEventScheduler> scheduler) {
    LOG_DEBUG("EventRaiserImpl: Setting EventScheduler for delayed event polling (WASM support)");
    scheduler_ = scheduler;

    // Recreate platform helper with scheduler support
    if (platformHelper_) {
        platformHelper_->shutdown();
    }
    platformHelper_ = createPlatformEventRaiserHelper(this, scheduler_);
    platformHelper_->start();

    LOG_DEBUG("EventRaiserImpl: EventScheduler set and platform helper reinitialized");
}

std::shared_ptr<IEventScheduler> EventRaiserImpl::getScheduler() const {
    return scheduler_;
}

void EventRaiserImpl::shutdown() {
    if (!isRunning_.load()) {
        return;  // Already shut down
    }

    LOG_DEBUG("EventRaiserImpl: Shutting down async processing");

    // Signal shutdown
    shutdownRequested_.store(true);

    // Zero Duplication Principle: Platform-specific shutdown through Helper
    if (platformHelper_) {
        platformHelper_->shutdown();
    }

    // MEMORY LEAK FIX: Explicitly clear all internal data structures
    // This ensures no pending events leak
    {
        std::lock_guard<std::mutex> queueLock(queueMutex_);
        std::lock_guard<std::mutex> syncQueueLock(synchronousQueueMutex_);

        // Clear async event queue by creating empty queue and swapping
        std::queue<QueuedEvent> emptyQueue;
        eventQueue_.swap(emptyQueue);

        // Clear synchronous priority queue by creating empty queue and swapping
        std::priority_queue<QueuedEvent, std::vector<QueuedEvent>, QueuedEventComparator> emptySyncQueue;
        synchronousQueue_.swap(emptySyncQueue);
    }

    isRunning_.store(false);
    LOG_DEBUG("EventRaiserImpl: Shutdown complete");
}

void EventRaiserImpl::setEventCallback(EventCallback callback) {
    std::lock_guard<std::mutex> lock(callbackMutex_);
    bool hadCallback = (eventCallback_ != nullptr);
    eventCallback_ = std::move(callback);
    bool hasCallback = (eventCallback_ != nullptr);
    LOG_DEBUG(
        "EventRaiserImpl: Callback status changed - EventRaiser: {}, previous: {}, current: {}, immediateMode: {}",
        (void *)this, hadCallback ? "set" : "none", hasCallback ? "set" : "none", immediateMode_.load());
}

void EventRaiserImpl::clearEventCallback() {
    std::lock_guard<std::mutex> lock(callbackMutex_);
    eventCallback_ = nullptr;
    LOG_DEBUG("EventRaiserImpl: Event callback cleared");
}

bool EventRaiserImpl::raiseEvent(const std::string &eventName, const std::string &eventData) {
    // Default to INTERNAL priority for backward compatibility (raise actions and #_internal targets)
    return raiseEventWithPriority(eventName, eventData, EventPriority::INTERNAL, "", "", "");
}

bool EventRaiserImpl::raiseInternalEvent(const std::string &eventName, const std::string &eventData) {
    // W3C SCXML 3.13: Internal events have higher priority than external events
    return raiseEventWithPriority(eventName, eventData, EventPriority::INTERNAL, "", "", "");
}

bool EventRaiserImpl::raiseExternalEvent(const std::string &eventName, const std::string &eventData) {
    // [EVENT ROUTING] Log when external event is raised (child receives event from parent)
    LOG_INFO("[EVENT ROUTING] EventRaiser receiving EXTERNAL event '{}' with data '{}'", eventName, eventData);

    // W3C SCXML 5.10: External events have lower priority than internal events (test 510)
    return raiseEventWithPriority(eventName, eventData, EventPriority::EXTERNAL, "", "", "");
}

bool EventRaiserImpl::raiseEvent(const std::string &eventName, const std::string &eventData,
                                 const std::string &originSessionId) {
    // W3C SCXML 6.4: Raise event with origin tracking for finalize support
    return raiseEventWithPriority(eventName, eventData, EventPriority::INTERNAL, originSessionId, "", "");
}

bool EventRaiserImpl::raiseEvent(const std::string &eventName, const std::string &eventData, const std::string &sendId,
                                 bool) {
    // W3C SCXML 5.10: Raise error event with sendid from failed send element
    return raiseEventWithPriority(eventName, eventData, EventPriority::INTERNAL, "", sendId, "");
}

bool EventRaiserImpl::raiseEvent(const std::string &eventName, const std::string &eventData,
                                 const std::string &originSessionId, const std::string &invokeId) {
    // W3C SCXML 5.10 test 338: Raise event with both origin and invoke ID tracking
    return raiseEventWithPriority(eventName, eventData, EventPriority::INTERNAL, originSessionId, "", invokeId, "");
}

bool EventRaiserImpl::raiseEvent(const std::string &eventName, const std::string &eventData,
                                 const std::string &originSessionId, const std::string &invokeId,
                                 const std::string &originType) {
    // [EVENT ROUTING] Entry point logging - track calls from InvokeEventTarget
    LOG_INFO("[EVENT ROUTING] EventRaiser::raiseEvent() ENTRY - event='{}', origin='{}', invokeId='{}', "
             "originType='{}', EventRaiser instance={}",
             eventName, originSessionId, invokeId, originType, (void *)this);

    // W3C SCXML 5.10: Raise event with full metadata (origin, invoke ID, and origintype)
    // W3C SCXML Test 230: Platform events (done.*, error.*) must be queued, not processed immediately
    // This prevents nested processing issues when child completes during parent transition
    EventPriority priority = EventPriority::INTERNAL;
    if (isPlatformEvent(eventName)) {
        priority = EventPriority::EXTERNAL;  // Force queueing for platform events
    }

    LOG_INFO("[EVENT ROUTING] EventRaiser::raiseEvent() calling raiseEventWithPriority() - priority={}",
             (priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"));

    return raiseEventWithPriority(eventName, eventData, priority, originSessionId, "", invokeId, originType);
}

bool EventRaiserImpl::raiseEventWithPriority(const std::string &eventName, const std::string &eventData,
                                             EventPriority priority, const std::string &originSessionId,
                                             const std::string &sendId, const std::string &invokeId,
                                             const std::string &originType, int64_t timestampNs) {
    LOG_INFO("[EVENT ROUTING] EventRaiser::raiseEventWithPriority() ENTRY - event='{}', priority={}, isRunning={}, "
             "immediateMode={}, EventRaiser instance={}",
             eventName, (priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"), isRunning_.load(),
             immediateMode_.load(), (void *)this);

    LOG_DEBUG("EventRaiserImpl::raiseEventWithPriority called - event: '{}', data: '{}', priority: {}, EventRaiser "
              "instance: {}",
              eventName, eventData, (priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"), (void *)this);

    if (!isRunning_.load()) {
        LOG_ERROR("[EVENT ROUTING] FAILED: EventRaiser is NOT RUNNING - cannot raise event '{}'", eventName);
        LOG_WARN("EventRaiserImpl: Cannot raise event '{}' - processor is shut down", eventName);
        return false;
    }

    LOG_INFO("[EVENT ROUTING] EventRaiser IS RUNNING - proceeding with event routing");

    // W3C SCXML compliance: Check if immediate mode is enabled
    // W3C SCXML Test 230: Platform events (done.*, error.*) must ALWAYS be queued
    // to prevent nested processing issues when child completes during parent transition
    // W3C SCXML 3.13: In interactive debugging, scheduler MANUAL mode overrides immediate mode
    // All events must be queued for step-by-step execution, even if immediate mode is enabled
    // W3C SCXML 5.9.2: EXTERNAL events must NOT bypass INTERNAL events in the queue
    // EXTERNAL events can use immediate mode only if no INTERNAL events are queued (Test 422)
    bool isSchedulerManual = scheduler_ && (scheduler_->getMode() == SchedulerMode::MANUAL);
    bool isPlatform = isPlatformEvent(eventName);
    bool isInternal = (priority == EventPriority::INTERNAL);
    bool hasInternalEvents = hasQueuedInternalEvents();

    if (immediateMode_.load() && !isPlatform && !isSchedulerManual) {
        // W3C SCXML 5.9.2: INTERNAL events always use immediate mode
        // EXTERNAL events use immediate mode only when INTERNAL queue is empty
        bool canProcessImmediately = isInternal || !hasInternalEvents;

        if (canProcessImmediately) {
            // Immediate processing allowed
            size_t queueSize = 0;
            {
                std::lock_guard<std::mutex> lock(synchronousQueueMutex_);
                queueSize = synchronousQueue_.size();
            }
            LOG_DEBUG("EventRaiserImpl: Processing {} event '{}' immediately (SCXML mode, hasInternalEvents={}, "
                      "queueSize={})",
                      (isInternal ? "INTERNAL" : "EXTERNAL"), eventName, hasInternalEvents, queueSize);

            // Get callback under lock
            EventCallback callback;
            {
                std::lock_guard<std::mutex> lock(callbackMutex_);
                callback = eventCallback_;
            }

            if (callback) {
                try {
                    // W3C SCXML 6.4: Set thread-local originSessionId before immediate callback execution
                    currentOriginSessionId_ = originSessionId;

                    // W3C SCXML 5.10: Set thread-local sendId before immediate callback execution
                    currentSendId_ = sendId;

                    // W3C SCXML 5.10: Set thread-local invokeId before immediate callback execution (test 338)
                    currentInvokeId_ = invokeId;

                    // W3C SCXML 5.10: Set thread-local originType before immediate callback execution (test 253, 331,
                    // 352, 372)
                    currentOriginType_ = originType;

                    bool result = callback(eventName, eventData);

                    // Clear after callback
                    currentOriginSessionId_.clear();
                    currentSendId_.clear();
                    currentInvokeId_.clear();
                    currentOriginType_.clear();

                    return result;
                } catch (const std::exception &e) {
                    LOG_ERROR("EventRaiserImpl: Exception in immediate processing: {}", e.what());
                    currentOriginSessionId_.clear();  // Ensure cleanup on exception
                    currentSendId_.clear();
                    currentInvokeId_.clear();
                    currentOriginType_.clear();
                    return false;
                }
            } else {
                LOG_WARN(
                    "EventRaiserImpl: No callback set for immediate event: {} - EventRaiser: {}, immediateMode: {}",
                    eventName, (void *)this, immediateMode_.load());
                return false;
            }
        }  // end if (canProcessImmediately)
    }  // end if (immediateMode_.load() && !isPlatform && !isSchedulerManual)

    // SCXML compliance: Use synchronous queue when immediate mode is disabled
    // W3C SCXML 5.9.2: EXTERNAL events queued when INTERNAL events are pending
    {
        std::lock_guard<std::mutex> lock(synchronousQueueMutex_);

        // W3C SCXML 3.13: Restore original timestamp for snapshot restoration (FIFO order preservation)
        std::chrono::steady_clock::time_point timestamp;
        if (timestampNs > 0) {
            // Restore from snapshot: use original timestamp
            timestamp = std::chrono::steady_clock::time_point(std::chrono::nanoseconds(timestampNs));
        } else {
            // New event: use current time
            timestamp = std::chrono::steady_clock::time_point();  // Will be set to now() in constructor
        }

        synchronousQueue_.emplace(eventName, eventData, priority, originSessionId, sendId, invokeId, originType,
                                  timestamp);

        // Enhanced logging: explain why event was queued instead of processed immediately
        std::string reason = "immediateMode disabled";
        if (immediateMode_.load()) {
            if (isPlatform) {
                reason = "platform event (done.*/error.*)";
            } else if (isSchedulerManual) {
                reason = "scheduler in MANUAL mode";
            } else if (!isInternal && hasInternalEvents) {
                reason = "EXTERNAL event blocked by INTERNAL events (W3C 5.9.2)";
            }
        }

        LOG_DEBUG("EventRaiserImpl: Event '{}' queued with priority {} (reason: {}) - queue size now: {}", eventName,
                  (priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"), reason, synchronousQueue_.size());
        LOG_DEBUG("EventRaiserImpl: Event '{}' queued for synchronous processing (SCXML compliance) with {} priority",
                  eventName, (priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"));
        LOG_DEBUG("EventRaiserImpl: Synchronous queue size after queueing: {}", synchronousQueue_.size());
    }

    // SCXML "fire and forget" - always return true for queuing
    return true;
}

bool EventRaiserImpl::isReady() const {
    std::lock_guard<std::mutex> lock(callbackMutex_);
    return eventCallback_ != nullptr && isRunning_.load();
}

void EventRaiserImpl::eventProcessingWorker() {
    LOG_DEBUG("EventRaiserImpl: Worker thread started");

    while (platformHelper_->shouldProcessEvents()) {
        // Zero Duplication Principle: Platform-specific wait logic through Helper
        platformHelper_->waitForEvents();

        std::unique_lock<std::mutex> lock(queueMutex_);

        // Process all queued events
        while (!eventQueue_.empty() && platformHelper_->shouldProcessEvents()) {
            QueuedEvent event = eventQueue_.front();
            eventQueue_.pop();

            // Release lock during event processing to prevent deadlock
            lock.unlock();

            // Process the event
            processEvent(event);

            // Reacquire lock for next iteration
            lock.lock();
        }
    }

    LOG_DEBUG("EventRaiserImpl: Worker thread stopped");
}

void EventRaiserImpl::processEvent(const QueuedEvent &event) {
    // Get callback under lock
    EventCallback callback;
    {
        std::lock_guard<std::mutex> lock(callbackMutex_);
        callback = eventCallback_;
    }

    if (!callback) {
        LOG_WARN("EventRaiserImpl: No callback set for event: {}", event.eventName);
        return;
    }

    try {
        LOG_DEBUG("EventRaiserImpl: Processing event '{}' with data: {}", event.eventName, event.eventData);

        // W3C SCXML 6.4 & 5.10: Set thread-local metadata before callback execution
        currentOriginSessionId_ = event.origin;
        currentSendId_ = event.sendId;
        currentInvokeId_ = event.invokeId;
        currentOriginType_ = event.originType;

        // Execute the callback (this is where the actual event processing happens)
        bool result = callback(event.eventName, event.eventData);

        // Clear thread-local metadata after callback
        currentOriginSessionId_.clear();
        currentSendId_.clear();
        currentInvokeId_.clear();
        currentOriginType_.clear();

        // SCXML "fire and forget": Log result but don't propagate failures
        // Event processing failures don't affect the async queue operation
        LOG_DEBUG("EventRaiserImpl: Event '{}' processed with result: {}", event.eventName, result);

    } catch (const std::exception &e) {
        LOG_ERROR("EventRaiserImpl: Exception while processing event '{}': {}", event.eventName, e.what());
        // Ensure cleanup on exception
        currentOriginSessionId_.clear();
        currentSendId_.clear();
        currentInvokeId_.clear();
        currentOriginType_.clear();
    }
}

void EventRaiserImpl::setImmediateMode(bool immediate) {
    immediateMode_.store(immediate);
    LOG_DEBUG("EventRaiserImpl: Immediate mode {}", immediate ? "enabled" : "disabled");
}

void EventRaiserImpl::processQueuedEvents() {
    LOG_DEBUG("EventRaiserImpl: Processing all queued events synchronously");

    // W3C SCXML 6.2: Poll EventScheduler for ready delayed events (platform-transparent)
    // Platform-specific behavior: WASM polls, Native no-op (background thread handles it)
    if (platformHelper_) {
        platformHelper_->pollScheduler();
    }

    // Process all currently queued synchronous events with W3C SCXML priority ordering
    std::vector<QueuedEvent> eventsToProcess;

    // Move all synchronous queued events to local vector under lock
    {
        std::lock_guard<std::mutex> lock(synchronousQueueMutex_);
        LOG_DEBUG("EventRaiserImpl: Synchronous queue size before processing: {}", synchronousQueue_.size());

        // W3C SCXML compliance: priority_queue already maintains priority order
        // Extract all events in priority order
        while (!synchronousQueue_.empty()) {
            eventsToProcess.push_back(synchronousQueue_.top());
            synchronousQueue_.pop();
        }

        LOG_DEBUG("EventRaiserImpl: Events extracted in priority order for processing: {}", eventsToProcess.size());
    }

    // Events are already in correct priority order from priority_queue
    LOG_DEBUG("EventRaiserImpl: Events already sorted by W3C SCXML priority (INTERNAL first, then EXTERNAL)");

    // [W3C193 DEBUG] Log the event processing order
    for (size_t i = 0; i < eventsToProcess.size(); ++i) {
        const auto &event = eventsToProcess[i];
        LOG_DEBUG("EventRaiserImpl: [W3C193 DEBUG] Event processing order[{}]: '{}' with priority {}", i,
                  event.eventName, (event.priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"));
    }

    // Process events without holding the queue lock
    for (const auto &event : eventsToProcess) {
        LOG_DEBUG("EventRaiserImpl: Synchronously processing queued event '{}' with {} priority", event.eventName,
                  (event.priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"));

        // Use common callback execution method
        executeEventCallback(event);
    }

    LOG_DEBUG("EventRaiserImpl: Finished processing all queued events");
}

bool EventRaiserImpl::processNextQueuedEvent() {
    LOG_DEBUG("EventRaiserImpl: Processing ONE queued event (W3C SCXML compliance)");

    // W3C SCXML 6.4: Get event from queue but DON'T remove yet
    // Finalize handler must execute BEFORE removing event from queue
    QueuedEvent eventToProcess{"", "", EventPriority::EXTERNAL};
    bool hasEvent = false;

    {
        std::lock_guard<std::mutex> lock(synchronousQueueMutex_);

        if (synchronousQueue_.empty()) {
            LOG_DEBUG("EventRaiserImpl: No queued events to process");
            return false;
        }

        // W3C SCXML compliance: Get highest priority event (INTERNAL before EXTERNAL)
        eventToProcess = synchronousQueue_.top();
        hasEvent = true;

        LOG_DEBUG(
            "EventRaiserImpl: Selected event '{}' with priority {} - {} events in queue", eventToProcess.eventName,
            (eventToProcess.priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"), synchronousQueue_.size());
    }

    if (!hasEvent) {
        return false;
    }

    // W3C SCXML 6.4: Execute callback (including finalize) BEFORE removing from queue
    bool success = executeEventCallback(eventToProcess);

    // W3C SCXML 6.4: Only NOW remove event from queue (after finalize executed)
    {
        std::lock_guard<std::mutex> lock(synchronousQueueMutex_);
        if (!synchronousQueue_.empty() && synchronousQueue_.top().eventName == eventToProcess.eventName) {
            synchronousQueue_.pop();
            LOG_DEBUG("EventRaiserImpl: Event '{}' removed from queue after processing", eventToProcess.eventName);
        }
    }

    return success;
}

bool EventRaiserImpl::executeEventCallback(const QueuedEvent &event) {
    // Get callback under lock
    EventCallback callback;
    {
        std::lock_guard<std::mutex> lock(callbackMutex_);
        callback = eventCallback_;
    }

    if (!callback) {
        LOG_WARN("EventRaiserImpl: No callback set for event: {}", event.eventName);
        return false;
    }

    try {
        LOG_DEBUG("EventRaiserImpl: Processing event '{}' with data '{}' from origin '{}'", event.eventName,
                  event.eventData, event.origin);

        // W3C SCXML 5.10.1: Store originSessionId in thread-local for StateMachine to access (_event.origin)
        currentOriginSessionId_ = event.origin;

        // W3C SCXML 5.10.1: Store sendId in thread-local for StateMachine to access (_event.sendid)
        currentSendId_ = event.sendId;

        // W3C SCXML 5.10.1: Store invokeId in thread-local for StateMachine to access (_event.invokeid)
        currentInvokeId_ = event.invokeId;

        // W3C SCXML 5.10.1: Store originType in thread-local for StateMachine to access (_event.origintype)
        currentOriginType_ = event.originType;

        // W3C SCXML 5.10.1: Store event type in thread-local for StateMachine to access (test 331)
        // ARCHITECTURE.md: Zero Duplication - Uses EventTypeHelper for Single Source of Truth
        bool isExternal = (event.priority == EventPriority::EXTERNAL);
        currentEventType_ = EventTypeHelper::classifyEventType(event.eventName, isExternal);

        // W3C SCXML 3.13: Store last processed event for time-travel debugging
        {
            std::lock_guard<std::mutex> lock(lastProcessedEventMutex_);
            lastProcessedEventName_ = event.eventName;
            lastProcessedEventData_ = event.eventData;
        }

        bool result = callback(event.eventName, event.eventData);

        // Clear after callback
        currentOriginSessionId_.clear();
        currentSendId_.clear();
        currentInvokeId_.clear();
        currentOriginType_.clear();
        currentEventType_.clear();
        LOG_DEBUG("EventRaiserImpl: Event '{}' processed with result: {}", event.eventName, result);
        return result;  // Return actual callback result (transition success/failure)
    } catch (const std::exception &e) {
        LOG_ERROR("EventRaiserImpl: Exception processing event '{}': {}", event.eventName, e.what());
        currentOriginSessionId_.clear();
        currentSendId_.clear();
        currentInvokeId_.clear();
        currentOriginType_.clear();
        currentEventType_.clear();
        return false;
    }
}

bool EventRaiserImpl::hasQueuedEvents() const {
    std::lock_guard<std::mutex> lock(synchronousQueueMutex_);
    return !synchronousQueue_.empty();
}

bool EventRaiserImpl::hasQueuedInternalEvents() const {
    // W3C SCXML 5.9.2: Check if INTERNAL priority events are in the queue
    // This is used to enforce event priority - EXTERNAL events should not bypass
    // INTERNAL events that are already queued
    std::lock_guard<std::mutex> lock(synchronousQueueMutex_);

    // Performance optimization: QueuedEventComparator ensures INTERNAL (priority 0)
    // events are always at the top of the queue before EXTERNAL (priority 1) events.
    // Therefore, we only need to check the top element instead of copying entire queue.
    // O(1) time complexity vs O(n) for full queue copy.
    if (synchronousQueue_.empty()) {
        return false;
    }

    return synchronousQueue_.top().priority == EventPriority::INTERNAL;
}

void EventRaiserImpl::getEventQueues(std::vector<EventSnapshot> &outInternal,
                                     std::vector<EventSnapshot> &outExternal) const {
    outInternal.clear();
    outExternal.clear();

    // W3C SCXML 3.13: Internal queue has higher priority than external queue
    // Copy synchronousQueue_ and separate by priority
    std::lock_guard<std::mutex> lock(synchronousQueueMutex_);

    // Priority queue doesn't support iteration, so copy to vector first
    auto queueCopy = synchronousQueue_;
    std::vector<QueuedEvent> allEvents;

    while (!queueCopy.empty()) {
        allEvents.push_back(queueCopy.top());
        queueCopy.pop();
    }

    // Separate by priority (INTERNAL vs EXTERNAL)
    // W3C SCXML 5.10.1: Capture complete event metadata for _event object restoration
    // W3C SCXML 3.13: Preserve timestamps for FIFO ordering during snapshot restore
    for (const auto &event : allEvents) {
        // Convert timestamp to nanoseconds since epoch for serialization
        int64_t timestampNs =
            std::chrono::duration_cast<std::chrono::nanoseconds>(event.timestamp.time_since_epoch()).count();

        EventSnapshot snapshot(event.eventName, event.eventData, event.sendId, event.originType, event.origin,
                               event.invokeId, timestampNs);

        if (event.priority == EventPriority::INTERNAL) {
            outInternal.push_back(snapshot);
        } else {
            outExternal.push_back(snapshot);
        }
    }

    LOG_DEBUG("EventRaiserImpl: Queue snapshot retrieved - internal: {}, external: {}", outInternal.size(),
              outExternal.size());
}

void EventRaiserImpl::clearQueue() {
    // W3C SCXML: Clear all queued events for time-travel debugging
    std::lock_guard<std::mutex> lock(synchronousQueueMutex_);

    // Count events before clearing for logging
    auto queueCopy = synchronousQueue_;
    size_t clearedCount = 0;
    while (!queueCopy.empty()) {
        clearedCount++;
        queueCopy.pop();
    }

    // Clear the queue by swapping with empty priority_queue
    std::priority_queue<QueuedEvent, std::vector<QueuedEvent>, QueuedEventComparator> emptyQueue;
    synchronousQueue_.swap(emptyQueue);

    LOG_DEBUG("EventRaiserImpl: Cleared {} queued events for state restoration", clearedCount);
}

bool EventRaiserImpl::getLastProcessedEvent(std::string &outEventName, std::string &outEventData) const {
    // W3C SCXML 3.13: Retrieve last processed event for time-travel debugging
    std::lock_guard<std::mutex> lock(lastProcessedEventMutex_);

    if (lastProcessedEventName_.empty()) {
        return false;
    }

    outEventName = lastProcessedEventName_;
    outEventData = lastProcessedEventData_;
    return true;
}

}  // namespace SCE