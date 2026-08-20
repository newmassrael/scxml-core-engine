// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "runtime/EventRaiserImpl.h"
#include "common/EventDataHelper.h"
#include "common/EventTypeHelper.h"
#include "common/IOProcessorHelper.h"
#include "common/StringUtils.h"
#include "core/EventMatchingHelper.h"
#include "core/LogMacros.h"
#include "events/IEventDispatcher.h"
#include "events/PlatformEventRaiserHelper.h"
#include "runtime/StateSnapshot.h"
#include <mutex>

namespace SCE {

// §scxml-5.10: Consolidated thread-local event context for callback execution
thread_local EventRaiserImpl::EventContext EventRaiserImpl::currentEventContext_;

EventRaiserImpl::EventRaiserImpl(EventCallback callback)
    : eventCallback_(std::move(callback)), scheduler_(nullptr), shutdownRequested_(false), isRunning_(false),
      immediateMode_(false) {
    SCE_LOG_DEBUG("EventRaiserImpl: Created with callback: {} (instance: {})", (eventCallback_ ? "set" : "none"),
                  (void *)this);

    // Zero Duplication Principle: Platform-specific initialization through Helper
    // Note: scheduler_ will be set later via setScheduler() for delayed event polling support
    platformHelper_ = createPlatformEventRaiserHelper(this, scheduler_);
    platformHelper_->start();

    SCE_LOG_DEBUG("EventRaiserImpl: Platform-specific initialization complete");
}

EventRaiserImpl::~EventRaiserImpl() {
    shutdown();
}

void EventRaiserImpl::setScheduler(std::shared_ptr<IEventScheduler> scheduler) {
    SCE_LOG_DEBUG("EventRaiserImpl: Setting EventScheduler for delayed event polling (WASM support)");
    scheduler_ = scheduler;

    // Recreate platform helper with scheduler support
    if (platformHelper_) {
        platformHelper_->shutdown();
    }

    // Reset shutdown flag so the new worker thread will run
    shutdownRequested_.store(false);

    platformHelper_ = createPlatformEventRaiserHelper(this, scheduler_);
    platformHelper_->start();

    SCE_LOG_DEBUG("EventRaiserImpl: EventScheduler set and platform helper reinitialized");
}

std::shared_ptr<IEventScheduler> EventRaiserImpl::getScheduler() const {
    return scheduler_;
}

size_t EventRaiserImpl::cancelEventsForSession(const std::string &originSessionId) {
    if (originSessionId.empty()) {
        SCE_LOG_WARN("EventRaiserImpl: Cannot cancel events for empty originSessionId");
        return 0;
    }

    std::lock_guard<std::mutex> lock(synchronousQueueMutex_);

    // §scxml-6.4.3: Remove all queued events from the specified session
    // priority_queue doesn't support direct removal, so we need to rebuild it
    std::vector<QueuedEvent> remaining;
    size_t cancelledCount = 0;

    // Extract all events from the queue
    while (!synchronousQueue_.empty()) {
        QueuedEvent event = synchronousQueue_.top();
        synchronousQueue_.pop();

        // Check if this event originated from the cancelled session. Both
        // sides are session ids: the queue carries what callers hand in, and
        // the §scxml-C-1 location is derived only where `_event.origin` is
        // published, so there is one spelling to compare here.
        if (event.origin == originSessionId) {
            cancelledCount++;
            SCE_LOG_DEBUG("EventRaiserImpl: Cancelled queued event '{}' from session: {}", event.eventName,
                          originSessionId);
        } else {
            // Keep events from other sessions
            remaining.push_back(event);
        }
    }

    // Rebuild the queue with remaining events
    for (const auto &event : remaining) {
        synchronousQueue_.push(event);
    }

    if (cancelledCount > 0) {
        SCE_LOG_INFO("EventRaiserImpl: Cancelled {} queued event(s) from session: {}", cancelledCount, originSessionId);
    }

    return cancelledCount;
}

void EventRaiserImpl::shutdown() {
    if (!isRunning_.load()) {
        return;  // Already shut down
    }

    SCE_LOG_DEBUG("EventRaiserImpl: Shutting down async processing");

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
    SCE_LOG_DEBUG("EventRaiserImpl: Shutdown complete");
}

void EventRaiserImpl::setEventCallback(EventCallback callback) {
    std::lock_guard<std::mutex> lock(callbackMutex_);
    bool hadCallback = (eventCallback_ != nullptr);
    eventCallback_ = std::move(callback);
    bool hasCallback = (eventCallback_ != nullptr);
    SCE_LOG_DEBUG(
        "EventRaiserImpl: Callback status changed - EventRaiser: {}, previous: {}, current: {}, immediateMode: {}",
        (void *)this, hadCallback ? "set" : "none", hasCallback ? "set" : "none", immediateMode_.load());
}

void EventRaiserImpl::clearEventCallback() {
    std::lock_guard<std::mutex> lock(callbackMutex_);
    eventCallback_ = nullptr;
    SCE_LOG_DEBUG("EventRaiserImpl: Event callback cleared");
}

bool EventRaiserImpl::raiseEvent(const std::string &eventName, const std::string &eventData) {
    // Default to INTERNAL priority for backward compatibility (raise actions and #_internal targets)
    return raiseEventWithPriority(eventName, eventData, EventPriority::INTERNAL, "", "", "");
}

bool EventRaiserImpl::raiseInternalEvent(const std::string &eventName, const std::string &eventData) {
    // §scxml-3.13: Internal events have higher priority than external events
    // §scxml-4.2.2: enqueueing appends at the rear, so events raised by <raise> keep
    // their arrival order behind whatever is already queued.
    return raiseEventWithPriority(eventName, eventData, EventPriority::INTERNAL, "", "", "");
}

bool EventRaiserImpl::raiseExternalEvent(const std::string &eventName, const std::string &eventData) {
    // [EVENT ROUTING] Log when external event is raised (child receives event from parent)
    SCE_LOG_INFO("[EVENT ROUTING] EventRaiser receiving EXTERNAL event '{}' with data '{}'", eventName, eventData);

    // §scxml-5.10: External events have lower priority than internal events (test 510)
    return raiseEventWithPriority(eventName, eventData, EventPriority::EXTERNAL, "", "", "");
}

bool EventRaiserImpl::raiseEvent(const std::string &eventName, const std::string &eventData,
                                 const std::string &originSessionId) {
    // §scxml-5.10: Raise event with origin tracking for finalize support
    return raiseEventWithPriority(eventName, eventData, EventPriority::INTERNAL, originSessionId, "", "");
}

bool EventRaiserImpl::raiseEvent(const std::string &eventName, const std::string &eventData, const std::string &sendId,
                                 bool) {
    // §scxml-5.10: Raise error event with sendid from failed send element
    return raiseEventWithPriority(eventName, eventData, EventPriority::INTERNAL, "", sendId, "");
}

bool EventRaiserImpl::raiseEvent(const std::string &eventName, const std::string &eventData,
                                 const std::string &originSessionId, const std::string &invokeId) {
    // §scxml-5.10 test 338: Raise event with both origin and invoke ID tracking
    return raiseEventWithPriority(eventName, eventData, EventPriority::INTERNAL, originSessionId, "", invokeId, "");
}

bool EventRaiserImpl::raiseEvent(const std::string &eventName, const std::string &eventData,
                                 const std::string &originSessionId, const std::string &invokeId,
                                 const std::string &originType) {
    // [EVENT ROUTING] Entry point logging - track calls from InvokeEventTarget
    SCE_LOG_INFO("[EVENT ROUTING] EventRaiser::raiseEvent() ENTRY - event='{}', origin='{}', invokeId='{}', "
                 "originType='{}', EventRaiser instance={}",
                 eventName, originSessionId, invokeId, originType, (void *)this);

    // §scxml-5.10: Raise event with full metadata (origin, invoke ID, and origintype)
    //
    // Queue membership answers one question only — which of the two queues
    // Appendix D's mainEventLoop drains this event from — and it is decided by
    // where the event came from, never by how it is named. An event carrying
    // another session's id is external (§scxml-5.10.1, W3C Test 252:
    // child->parent). Everything the processor raises for itself is internal:
    // §scxml-3.12.2 signals an error with an `error.*` event placed on the
    // queue and processed like any other — never delivered inline, and simply
    // dropped when no transition matches it — and 3.7 puts `done.state.<id>`
    // there too. `done.invoke.<id>` is external and stays so without a name
    // test, because 6.4.2 has it returned by the invoked service —
    // `InvokeExecutor` raises it with the child's session id as origin, which
    // is exactly the branch below.
    //
    // Naming the done/error families here once forced them external for an
    // unrelated reason: to stop them being delivered inline (W3C Test 230 —
    // a child completing mid-transition must not re-enter the parent). That
    // is a re-entrancy property, not a queue, and it has its own guard in
    // `raiseEventWithPriority` below, where `isPlatformEvent` gates immediate
    // mode directly. Conflating the two put `error.*` and `done.state.*` on
    // the external queue, where the autoforward step hands whatever it
    // dequeues to every `autoforward` child — events the spec never forwards,
    // reaching children that must not see them.
    EventPriority priority = EventPriority::INTERNAL;
    if (!originSessionId.empty()) {
        // §scxml-5.10.1: Events from other SCXML sessions are EXTERNAL
        // This ensures correct event priority ordering (FIFO within same priority)
        priority = EventPriority::EXTERNAL;
    }

    SCE_LOG_INFO("[EVENT ROUTING] EventRaiser::raiseEvent() calling raiseEventWithPriority() - priority={}",
                 (priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"));

    return raiseEventWithPriority(eventName, eventData, priority, originSessionId, "", invokeId, originType);
}

bool EventRaiserImpl::raiseEventWithPriority(const std::string &eventName, const std::string &eventData,
                                             EventPriority priority, const std::string &originSessionId,
                                             const std::string &sendId, const std::string &invokeId,
                                             const std::string &originType, int64_t timestampNs,
                                             std::optional<ScriptValue> typedData) {
    // §scxml-B-2: Parse JSON eventData to ScriptValue at pipeline level (engine-agnostic)
    // Covers all entry points: EventTargets, HTTP callbacks, AOT, DoneData
    if (!typedData.has_value() && !eventData.empty()) {
        typedData = EventDataHelper::jsonStringToScriptValue(eventData);
    }

    // §scxml-C-1 requires `_event.origin` to be the SENDER's published
    // `_ioprocessors` location rather than its bare session id, and this used
    // to convert here, on the way in. It cannot: the value this carries has
    // two consumers that need different spellings. `_event.origin` wants the
    // location; `getFinalizeScriptForChildSession` and
    // `shouldFilterCancelledInvokeEvent` look sessions up by the id they were
    // registered under. Converting at the entrance served the first and broke
    // the second silently — a `<finalize>` handler stopped running because
    // `sce://scxml/<id>` is not a key any registry holds (W3C 233/234).
    //
    // So the pipeline carries the SESSION ID, which is what callers hand in
    // and what every lookup keys on, and the location is derived at the one
    // place it is published (`ActionExecutorImpl::setCurrentEvent`). One
    // spelling per consumer, each produced where it is used.

    SCE_LOG_INFO("[EVENT ROUTING] EventRaiser::raiseEventWithPriority() ENTRY - event='{}', priority={}, isRunning={}, "
                 "immediateMode={}, EventRaiser instance={}",
                 eventName, (priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"), isRunning_.load(),
                 immediateMode_.load(), (void *)this);

    SCE_LOG_DEBUG("EventRaiserImpl::raiseEventWithPriority called - event: '{}', data: '{}', priority: {}, EventRaiser "
                  "instance: {}",
                  eventName, eventData, (priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"), (void *)this);

    if (!isRunning_.load()) {
        SCE_LOG_ERROR("[EVENT ROUTING] FAILED: EventRaiser is NOT RUNNING - cannot raise event '{}'", eventName);
        SCE_LOG_WARN("EventRaiserImpl: Cannot raise event '{}' - processor is shut down", eventName);
        return false;
    }

    // §scxml-3.12.2: the processor raises `error.*` events, and the clause
    // bounds what happens to one nothing matches. It says nothing about one a
    // handler DOES match and answers with the same failure — the failure
    // raises the error, the same transition answers it, and this raiser is
    // dispatched into again from inside its own dispatch, forever. Measured
    // 2026-08-19: `processEvent` never came back. So the chain is cut here,
    // and `getErrorCascadeEvents()` is how the host learns it was.
    //
    // Only the engine's own error events are refused: an author's `<raise>`
    // inside an error handler is the document doing its job.
    if (handlingErrorEvent_.load() && SCE::Core::EventMatchingHelper::isErrorEvent(eventName)) {
        if (errorCascadeDepth_.fetch_add(1) + 1 >= MAX_ERROR_CASCADE_DEPTH) {
            const uint32_t refused = errorCascadeEvents_.fetch_add(1) + 1;
            {
                std::lock_guard<std::mutex> lock(lastErrorCascadeEventMutex_);
                lastErrorCascadeEvent_ = eventName;
            }
            if (refused == 1) {
                SCE_LOG_ERROR("EventRaiserImpl: an error handler has raised an error {} times over; refusing to "
                              "feed the chain - the document's error handling is failing",
                              MAX_ERROR_CASCADE_DEPTH);
            }
            // Fire-and-forget, exactly as a queued raise reports: the caller
            // asked for an event to be delivered and this raiser owns whether
            // it is, which is the same contract every other refusal here has.
            return true;
        }
    }

    SCE_LOG_INFO("[EVENT ROUTING] EventRaiser IS RUNNING - proceeding with event routing");

    // W3C SCXML compliance: Check if immediate mode is enabled
    // W3C SCXML Test 230: Platform events (done.*, error.*) must ALWAYS be queued
    // to prevent nested processing issues when child completes during parent transition
    // In interactive debugging, scheduler MANUAL mode overrides immediate mode
    // All events must be queued for step-by-step execution, even if immediate mode is enabled
    // §scxml-3.13: EXTERNAL events must NOT bypass INTERNAL events in the queue
    // EXTERNAL events can use immediate mode only if no INTERNAL events are queued (Test 422)
    bool isSchedulerManual = scheduler_ && (scheduler_->getMode() == SchedulerMode::MANUAL);
    bool isPlatform = isPlatformEvent(eventName);
    bool isInternal = (priority == EventPriority::INTERNAL);
    bool hasInternalEvents = hasQueuedInternalEvents();

    if (immediateMode_.load() && !isPlatform && !isSchedulerManual) {
        // §scxml-3.13: INTERNAL events always use immediate mode
        //
        // §scxml-D-mainEventLoop: an EXTERNAL event may skip the queue only
        // when there is nothing it would jump ahead of. Testing the INTERNAL
        // queue alone is not that condition — an EXTERNAL event already queued
        // is equally entitled to be processed first, and letting a later
        // arrival overtake it breaks the run-to-completion order the loop
        // exists to impose. That is observable from a second session: a child
        // started by `<invoke>` sends to `#_parent` while an event the parent
        // queued for itself on the way in is still waiting, and the parent
        // ends up acting on the child's report before it has forwarded the
        // event the child was supposed to see.
        bool canProcessImmediately = isInternal || !hasQueuedEvents();

        if (canProcessImmediately) {
            // Immediate processing allowed
            size_t queueSize = 0;
            {
                std::lock_guard<std::mutex> lock(synchronousQueueMutex_);
                queueSize = synchronousQueue_.size();
            }
            SCE_LOG_DEBUG("EventRaiserImpl: Processing {} event '{}' immediately (SCXML mode, hasInternalEvents={}, "
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
                    // §scxml-5.10: RAII guard sets all event context fields and clears on scope exit
                    EventContext ctx;
                    ctx.originSessionId = originSessionId;
                    ctx.sendId = sendId;
                    ctx.invokeId = invokeId;
                    ctx.originType = originType;
                    ctx.eventType = EventTypeHelper::classifyEventType(eventName, !isInternal);
                    ctx.typedData = typedData;
                    ctx.isExternalQueue = !isInternal;
                    EventContextGuard guard(ctx);

                    // The same chain bookkeeping the queued path does below:
                    // an error raised while this dispatch runs was raised by
                    // the handler answering an error. Saved and restored
                    // rather than cleared — executable content dispatches into
                    // this raiser again, so these calls nest.
                    ErrorChainScope chain(*this, eventName);
                    bool result = callback(eventName, eventData);
                    return result;
                } catch (const std::exception &e) {
                    SCE_LOG_ERROR("EventRaiserImpl: Exception in immediate processing: {}", e.what());
                    return false;
                }
            } else {
                SCE_LOG_WARN(
                    "EventRaiserImpl: No callback set for immediate event: {} - EventRaiser: {}, immediateMode: {}",
                    eventName, (void *)this, immediateMode_.load());
                return false;
            }
        }  // end if (canProcessImmediately)
    }  // end if (immediateMode_.load() && !isPlatform && !isSchedulerManual)

    // SCXML compliance: Use synchronous queue when immediate mode is disabled
    // §scxml-3.13: EXTERNAL events queued when INTERNAL events are pending
    {
        std::lock_guard<std::mutex> lock(synchronousQueueMutex_);

        // Restore original timestamp for snapshot restoration (FIFO order preservation)
        std::chrono::steady_clock::time_point timestamp;
        if (timestampNs > 0) {
            // Restore from snapshot: use original timestamp
            timestamp = std::chrono::steady_clock::time_point(std::chrono::nanoseconds(timestampNs));
        } else {
            // New event: use current time
            timestamp = std::chrono::steady_clock::time_point();  // Will be set to now() in constructor
        }

        synchronousQueue_.emplace(eventName, eventData, priority, originSessionId, sendId, invokeId, originType,
                                  timestamp, std::move(typedData));

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

        SCE_LOG_DEBUG("EventRaiserImpl: Event '{}' queued with priority {} (reason: {}) - queue size now: {}",
                      eventName, (priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"), reason,
                      synchronousQueue_.size());
        SCE_LOG_DEBUG(
            "EventRaiserImpl: Event '{}' queued for synchronous processing (SCXML compliance) with {} priority",
            eventName, (priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"));
        SCE_LOG_DEBUG("EventRaiserImpl: Synchronous queue size after queueing: {}", synchronousQueue_.size());
    }

    // SCXML "fire and forget" - always return true for queuing
    return true;
}

EventRaiserImpl::ErrorChainScope::ErrorChainScope(EventRaiserImpl &raiser, const std::string &eventName)
    : raiser_(raiser) {
    const bool isError = SCE::Core::EventMatchingHelper::isErrorEvent(eventName);
    // Dispatching anything else does NOT end the chain. An earlier draft reset
    // the depth here on every non-error event, which reads as the careful
    // choice and is the opposite: a handler that raises its own event before
    // failing — a document that logs, then fails, which is most of them —
    // leaves the queue alternating `tick, error, tick, error…`, and each
    // `tick` put the ceiling back out of reach. The count needs no such guard,
    // because it only ever rises while an error handler is running.
    raiser_.dispatchDepth_.fetch_add(1);
    previous_ = raiser_.handlingErrorEvent_.exchange(isError);
}

EventRaiserImpl::ErrorChainScope::~ErrorChainScope() {
    raiser_.handlingErrorEvent_.store(previous_);
    raiser_.dispatchDepth_.fetch_sub(1);
}

void EventRaiserImpl::resetErrorCascadeDepth() {
    // §scxml-3.12.2: the host has called in again, so the chain the last call
    // built is over. Only the depth — `errorCascadeEvents_` is what the host
    // reads and is a fact about the past.
    errorCascadeDepth_.store(0);
}

void EventRaiserImpl::setMicrostepBudget(MicrostepBudget budget) {
    std::lock_guard<std::mutex> lock(microstepBudgetMutex_);
    microstepBudget_ = std::move(budget);
}

bool EventRaiserImpl::mayTakeMicrostep() {
    std::function<bool()> mayTake;
    {
        std::lock_guard<std::mutex> lock(microstepBudgetMutex_);
        mayTake = microstepBudget_.mayTake;
    }
    // §scxml-3.13: no budget lent means no macrostep to bound — a raiser used
    // on its own dispatches exactly as it always did.
    return !mayTake || mayTake();
}

void EventRaiserImpl::spendMicrostep() {
    std::function<void()> spend;
    {
        std::lock_guard<std::mutex> lock(microstepBudgetMutex_);
        spend = microstepBudget_.spend;
    }
    if (spend) {
        spend();
    }
}

uint32_t EventRaiserImpl::getErrorCascadeEvents() const {
    // §scxml-3.12.2: the clause covers the error nobody answers; this counts
    // the error answered by a handler that fails the same way every time,
    // which the clause does not reach and which this raiser had to end.
    return errorCascadeEvents_.load();
}

std::string EventRaiserImpl::getLastErrorCascadeEvent() const {
    std::lock_guard<std::mutex> lock(lastErrorCascadeEventMutex_);
    return lastErrorCascadeEvent_;
}

bool EventRaiserImpl::isReady() const {
    std::lock_guard<std::mutex> lock(callbackMutex_);
    return eventCallback_ != nullptr && isRunning_.load();
}

void EventRaiserImpl::eventProcessingWorker() {
    SCE_LOG_DEBUG("EventRaiserImpl: Worker thread started");

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

    SCE_LOG_DEBUG("EventRaiserImpl: Worker thread stopped");
}

void EventRaiserImpl::processEvent(const QueuedEvent &event) {
    // Get callback under lock
    EventCallback callback;
    {
        std::lock_guard<std::mutex> lock(callbackMutex_);
        callback = eventCallback_;
    }

    if (!callback) {
        SCE_LOG_WARN("EventRaiserImpl: No callback set for event: {}", event.eventName);
        return;
    }

    try {
        SCE_LOG_DEBUG("EventRaiserImpl: Processing event '{}' with data: {}", event.eventName, event.eventData);

        // §scxml-5.10: RAII guard sets all event context fields and clears on scope exit
        bool isExternal = (event.priority == EventPriority::EXTERNAL);
        EventContext ctx;
        ctx.originSessionId = event.origin;
        ctx.sendId = event.sendId;
        ctx.invokeId = event.invokeId;
        ctx.originType = event.originType;
        ctx.eventType = EventTypeHelper::classifyEventType(event.eventName, isExternal);
        ctx.typedData = event.typedData;
        ctx.isExternalQueue = isExternal;
        EventContextGuard guard(ctx);

        bool result = callback(event.eventName, event.eventData);
        SCE_LOG_DEBUG("EventRaiserImpl: Event '{}' processed with result: {}", event.eventName, result);

    } catch (const std::exception &e) {
        SCE_LOG_ERROR("EventRaiserImpl: Exception while processing event '{}': {}", event.eventName, e.what());
    }
}

void EventRaiserImpl::setImmediateMode(bool immediate) {
    immediateMode_.store(immediate);
    SCE_LOG_DEBUG("EventRaiserImpl: Immediate mode {}", immediate ? "enabled" : "disabled");
}

void EventRaiserImpl::processQueuedEvents() {
    // Hot path: fires per microstep with ~empty queue on most calls (2k+ hits
    // per W3C harness run). Trace-only so Debug-level logs stay focused on
    // state-machine events, not event-loop plumbing.
    SCE_LOG_TRACE("EventRaiserImpl: Processing all queued events synchronously");

    // §scxml-6.2: Poll EventScheduler for ready delayed events (platform-transparent)
    // Platform-specific behavior: WASM polls, Native no-op (background thread handles it)
    if (platformHelper_) {
        platformHelper_->pollScheduler();
    }

    // Process all currently queued synchronous events with W3C SCXML priority ordering
    std::vector<QueuedEvent> eventsToProcess;

    // Move all synchronous queued events to local vector under lock
    {
        std::lock_guard<std::mutex> lock(synchronousQueueMutex_);
        SCE_LOG_TRACE("EventRaiserImpl: Synchronous queue size before processing: {}", synchronousQueue_.size());

        // W3C SCXML compliance: priority_queue already maintains priority order
        // Extract all events in priority order
        while (!synchronousQueue_.empty()) {
            eventsToProcess.push_back(synchronousQueue_.top());
            synchronousQueue_.pop();
        }

        SCE_LOG_TRACE("EventRaiserImpl: Events extracted in priority order for processing: {}", eventsToProcess.size());
    }

    // Events are already in correct priority order from priority_queue
    SCE_LOG_TRACE("EventRaiserImpl: Events already sorted by W3C SCXML priority (INTERNAL first, then EXTERNAL)");

    // [W3C193 DEBUG] Log the event processing order
    for (size_t i = 0; i < eventsToProcess.size(); ++i) {
        const auto &event = eventsToProcess[i];
        SCE_LOG_DEBUG("EventRaiserImpl: [W3C193 DEBUG] Event processing order[{}]: '{}' with priority {}", i,
                      event.eventName, (event.priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"));
    }

    // Process events without holding the queue lock
    for (size_t i = 0; i < eventsToProcess.size(); ++i) {
        const auto &event = eventsToProcess[i];
        SCE_LOG_DEBUG("EventRaiserImpl: Synchronously processing queued event '{}' with {} priority", event.eventName,
                      (event.priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"));

        // §scxml-3.13: the macrostep may have run out of budget. Put back
        // everything not yet dispatched — this drain took the whole queue into
        // a local vector, so "leave it queued" means returning it — and stop.
        // The next macrostep starts where this one was cut, which is what
        // makes the ceiling a pause rather than a loss. See `MicrostepBudget`.
        if (event.priority == EventPriority::INTERNAL && !mayTakeMicrostep()) {
            std::lock_guard<std::mutex> lock(synchronousQueueMutex_);
            for (size_t j = i; j < eventsToProcess.size(); ++j) {
                synchronousQueue_.push(eventsToProcess[j]);
            }
            SCE_LOG_DEBUG("EventRaiserImpl: microstep budget spent, {} event(s) left queued",
                          eventsToProcess.size() - i);
            break;
        }

        // Use common callback execution method
        const bool tookTransition = executeEventCallback(event);
        if (tookTransition && event.priority == EventPriority::INTERNAL) {
            spendMicrostep();
        }
    }

    SCE_LOG_TRACE("EventRaiserImpl: Finished processing all queued events");
}

bool EventRaiserImpl::processNextQueuedEvent() {
    SCE_LOG_DEBUG("EventRaiserImpl: Processing ONE queued event (W3C SCXML compliance)");

    // §scxml-D-mainEventLoop: dequeue, *then* dispatch. The algorithm removes
    // the event before it runs `applyFinalize` and selects transitions, and
    // §scxml-6.5.2's "right before it removes the event from the event queue
    // for processing" is still honoured because `<finalize>` runs inside the
    // callback below, ahead of transition selection — so the ordering holds
    // without leaving the event in the queue across the dispatch.
    //
    // Leaving it there is not a smaller change, it is a different algorithm.
    // Executable content dispatched from a transition re-enters this drain
    // (`<send>` -> dispatcher -> processQueuedEvents), and a re-entrant call
    // sees the event still at the head and processes it a second time.
    // Removal afterwards then compared `top().eventName` against the name it
    // started with, which is not identity: once the nested pass had changed
    // the head, the compare failed and the event was never removed at all.
    // One `<send>` inside a targetless transition was enough to reprocess a
    // single event hundreds of times while the event it raised never ran.
    QueuedEvent eventToProcess{"", "", EventPriority::EXTERNAL};
    bool isInternal = false;

    {
        std::lock_guard<std::mutex> lock(synchronousQueueMutex_);

        if (synchronousQueue_.empty()) {
            SCE_LOG_DEBUG("EventRaiserImpl: No queued events to process");
            return false;
        }

        // §scxml-3.13: an internal event is a microstep of the macrostep now
        // in progress, and this one may have none left. Asked before the pop,
        // so the refusal leaves the event queued. See `MicrostepBudget`.
        isInternal = synchronousQueue_.top().priority == EventPriority::INTERNAL;

        // Copied out and re-read below for the internal case: the budget gate
        // runs without this lock, because the machine that owns the budget may
        // reach back in.
        eventToProcess = synchronousQueue_.top();
        if (!isInternal) {
            // W3C SCXML compliance: Get highest priority event (INTERNAL before EXTERNAL)
            synchronousQueue_.pop();
            SCE_LOG_DEBUG("EventRaiserImpl: Dequeued EXTERNAL event '{}' - {} events left in queue",
                          eventToProcess.eventName, synchronousQueue_.size());
        }
    }

    // Dispatch happens with the queue lock RELEASED, on both paths.
    //
    // The callback runs the state machine, and a transition it selects can
    // reach straight back into this object: exiting a state cancels that
    // state's `<invoke>`, and cancelling an invoke calls
    // `cancelEventsForSession`, which takes this same mutex. `std::mutex` is
    // not recursive, so dispatching while holding it deadlocked the calling
    // thread against itself — every thread parked in `futex_wait`, no
    // progress, no diagnostic. Measured on
    // `DonedataLocalInvokeTest.ParentObservesDonedataOnDoneInvoke`, whose
    // `done.invoke.inv_param` arrives as an EXTERNAL event and whose
    // transition exits the state that owns the invoke.
    //
    // The internal path below already dispatched outside the lock; this is the
    // external one saying the same thing rather than a second arrangement of
    // it.
    if (!isInternal) {
        return executeEventCallback(eventToProcess);
    }

    if (!mayTakeMicrostep()) {
        return false;
    }
    {
        std::lock_guard<std::mutex> lock(synchronousQueueMutex_);
        if (synchronousQueue_.empty()) {
            return false;
        }
        eventToProcess = synchronousQueue_.top();
        synchronousQueue_.pop();
        SCE_LOG_DEBUG(
            "EventRaiserImpl: Dequeued event '{}' with priority {} - {} events left in queue", eventToProcess.eventName,
            (eventToProcess.priority == EventPriority::INTERNAL ? "INTERNAL" : "EXTERNAL"), synchronousQueue_.size());
    }

    const bool tookTransition = executeEventCallback(eventToProcess);
    if (tookTransition && eventToProcess.priority == EventPriority::INTERNAL) {
        spendMicrostep();
    }
    return tookTransition;
}

bool EventRaiserImpl::executeEventCallback(const QueuedEvent &event) {
    // Get callback under lock
    EventCallback callback;
    {
        std::lock_guard<std::mutex> lock(callbackMutex_);
        callback = eventCallback_;
    }

    if (!callback) {
        SCE_LOG_WARN("EventRaiserImpl: No callback set for event: {}", event.eventName);
        return false;
    }

    try {
        SCE_LOG_DEBUG("EventRaiserImpl: Processing event '{}' with data '{}' from origin '{}'", event.eventName,
                      event.eventData, event.origin);

        // §scxml-5.10: RAII guard sets all event context fields and clears on scope exit
        bool isExternal = (event.priority == EventPriority::EXTERNAL);
        EventContext ctx;
        ctx.originSessionId = event.origin;
        ctx.sendId = event.sendId;
        ctx.invokeId = event.invokeId;
        ctx.originType = event.originType;
        ctx.eventType = EventTypeHelper::classifyEventType(event.eventName, isExternal);
        ctx.typedData = event.typedData;
        ctx.isExternalQueue = isExternal;
        EventContextGuard guard(ctx);

        // Store last processed event for time-travel debugging
        {
            std::lock_guard<std::mutex> lock(lastProcessedEventMutex_);
            lastProcessedEventName_ = event.eventName;
            lastProcessedEventData_ = event.eventData;
        }

        // An error raised from here on is raised *by an error handler*, which
        // is the one situation this raiser cannot leave to the document: the
        // handler that failed is the same one that will answer the failure.
        // The scope is what `raiseEventWithPriority` reads to tell that apart
        // from a first failure, and it restores rather than clears because a
        // transition's executable content dispatches through here again.
        ErrorChainScope chain(*this, event.eventName);
        bool result = callback(event.eventName, event.eventData);
        SCE_LOG_DEBUG("EventRaiserImpl: Event '{}' processed with result: {}", event.eventName, result);
        return result;  // Return actual callback result (transition success/failure)
    } catch (const std::exception &e) {
        SCE_LOG_ERROR("EventRaiserImpl: Exception processing event '{}': {}", event.eventName, e.what());
        return false;
    }
}

bool EventRaiserImpl::hasQueuedEvents() const {
    std::lock_guard<std::mutex> lock(synchronousQueueMutex_);
    return !synchronousQueue_.empty();
}

bool EventRaiserImpl::hasQueuedInternalEvents() const {
    // §scxml-3.13: Check if INTERNAL priority events are in the queue
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

bool EventRaiserImpl::processNextInternalEvent() {
    // §scxml-D-mainEventLoop: the macrostep completes on internal events
    // alone. Popping an external event here would run it before the invokes
    // that the macrostep just armed, and an `autoforward` child would never
    // see it — so the pop is conditional on the head's class, not merely on
    // the queue being non-empty.
    QueuedEvent eventToProcess{"", "", EventPriority::EXTERNAL};

    {
        std::lock_guard<std::mutex> lock(synchronousQueueMutex_);

        // QueuedEventComparator keeps INTERNAL (priority 0) ahead of EXTERNAL,
        // so the head alone decides whether an internal event is available.
        if (synchronousQueue_.empty() || synchronousQueue_.top().priority != EventPriority::INTERNAL) {
            return false;
        }

        eventToProcess = synchronousQueue_.top();

        SCE_LOG_DEBUG("EventRaiserImpl: Dequeued INTERNAL event '{}' - {} events left in queue",
                      eventToProcess.eventName, synchronousQueue_.size() - 1);
    }

    // §scxml-3.13: asked before the event leaves the queue, so a refusal
    // leaves it for the next macrostep rather than swallowing it. See
    // `MicrostepBudget`.
    if (!mayTakeMicrostep()) {
        return false;
    }
    {
        std::lock_guard<std::mutex> lock(synchronousQueueMutex_);
        // The gate above released the lock, so re-read the head rather than
        // trusting the copy: a nested dispatch may have drained it meanwhile.
        if (synchronousQueue_.empty() || synchronousQueue_.top().priority != EventPriority::INTERNAL) {
            return false;
        }
        eventToProcess = synchronousQueue_.top();
        synchronousQueue_.pop();
    }

    const bool tookTransition = executeEventCallback(eventToProcess);
    if (tookTransition) {
        spendMicrostep();
    }
    return tookTransition;
}

void EventRaiserImpl::getEventQueues(std::vector<EventSnapshot> &outInternal,
                                     std::vector<EventSnapshot> &outExternal) const {
    outInternal.clear();
    outExternal.clear();

    // §scxml-3.13: Internal queue has higher priority than external queue
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
    // §scxml-5.10.1: Capture complete event metadata for _event object restoration
    // Preserve timestamps for FIFO ordering during snapshot restore
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

    SCE_LOG_DEBUG("EventRaiserImpl: Queue snapshot retrieved - internal: {}, external: {}", outInternal.size(),
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

    SCE_LOG_DEBUG("EventRaiserImpl: Cleared {} queued events for state restoration", clearedCount);
}

bool EventRaiserImpl::getLastProcessedEvent(std::string &outEventName, std::string &outEventData) const {
    // Retrieve last processed event for time-travel debugging
    std::lock_guard<std::mutex> lock(lastProcessedEventMutex_);

    if (lastProcessedEventName_.empty()) {
        return false;
    }

    outEventName = lastProcessedEventName_;
    outEventData = lastProcessedEventData_;
    return true;
}

}  // namespace SCE