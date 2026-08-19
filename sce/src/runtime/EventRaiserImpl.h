// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "IEventRaiser.h"
#include "SCXMLTypes.h"
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <queue>

namespace SCE {

// Forward declarations
class PlatformEventRaiserHelper;
class SynchronousEventRaiserHelper;
class QueuedEventRaiserHelper;
class IEventScheduler;

/**
 * @brief SCXML-compliant asynchronous implementation of IEventRaiser
 *
 * This class implements the SCXML "fire and forget" event model using
 * asynchronous event queues to prevent deadlocks and ensure proper
 * event processing order as specified by W3C SCXML standard.
 *
 * Lock ordering (always acquire in this order to prevent deadlocks):
 *   1. queueMutex_              (async event queue)
 *   2. synchronousQueueMutex_   (SCXML mode synchronous queue)
 *   3. callbackMutex_           (event callback registration)
 *   4. lastProcessedEventMutex_ (debug/snapshot state)
 */
class EventRaiserImpl : public IEventRaiser {
    // Forward declarations for EventScheduler support
    friend class IEventScheduler;

    // Allow PlatformEventRaiserHelper and its implementations to access private members
    friend class PlatformEventRaiserHelper;
    friend class SynchronousEventRaiserHelper;
    friend class QueuedEventRaiserHelper;
    friend std::unique_ptr<PlatformEventRaiserHelper> createPlatformEventRaiserHelper(EventRaiserImpl *,
                                                                                      std::shared_ptr<IEventScheduler>);

public:
    /**
     * @brief How many links an `error.*` chain may have before this raiser
     *        stops feeding it — see `getErrorCascadeEvents()`
     *
     * §scxml-3.12.2 says what to do with an error event nothing matches. It
     * does not say what to do when something *does* match it and that handler
     * fails too, so the number is this engine's to choose, and it matches the
     * ceiling `EventProcessingAlgorithms::checkEventlessTransitions` uses for
     * the sibling case of a macrostep that cannot finish.
     *
     * A hundred links is far past any repair strategy a document plausibly
     * spells (a handler that tries a fallback, then a second one, is three)
     * and far short of a number a host would wait through. Here it also caps
     * recursion: executable content runs a nested drain, so each link is a
     * stack frame.
     */
    static constexpr uint32_t MAX_ERROR_CASCADE_DEPTH = 100;

    using EventCallback = std::function<bool(const std::string &, const std::string &)>;
    using EventCallbackWithOrigin = std::function<bool(const std::string &, const std::string &, const std::string &)>;

    /**
     * @brief W3C SCXML event priority for queue processing
     */
    enum class EventPriority {
        INTERNAL = 0,  // High priority - internal queue events (raise, send with target="#_internal")
        EXTERNAL = 1   // Low priority - external queue events (send without target or with external targets)
    };

    /**
     * @brief Thread-local event context for §scxml-5.10 metadata passing
     *
     * Consolidates all thread-local variables into a single struct.
     * Set before event callback execution, cleared after callback returns.
     * StateMachine reads this during processEvent() to populate _event fields.
     */
    struct EventContext {
        std::string originSessionId;           // §scxml-5.10: _event.origin
        std::string sendId;                    // §scxml-5.10: _event.sendid
        std::string invokeId;                  // §scxml-5.10: _event.invokeid
        std::string originType;                // §scxml-5.10: _event.origintype
        std::string eventType;                 // §scxml-5.10: "internal"/"platform"/"external"
        std::optional<ScriptValue> typedData;  // Engine-agnostic typed data (avoids JSON round-trip)
        // W3C SCXML Appendix D mainEventLoop: which of the two queues this
        // event was dequeued from. Distinct from `eventType`, which is the
        // 5.10.1 `_event.type` string and reads "platform" for the done and
        // error families whichever queue carried them. Consumers that must
        // act only at the external-dequeue point — autoforward is the one
        // the spec names — key on this, never on the event's name.
        bool isExternalQueue = false;

        void clear() {
            originSessionId.clear();
            sendId.clear();
            invokeId.clear();
            originType.clear();
            eventType.clear();
            typedData.reset();
            isExternalQueue = false;
        }
    };

    /**
     * @brief RAII guard for thread-local EventContext
     * Sets context on construction, clears on destruction (exception-safe).
     */
    struct EventContextGuard {
        explicit EventContextGuard(EventContext ctx) {
            currentEventContext_ = std::move(ctx);
        }

        ~EventContextGuard() {
            currentEventContext_.clear();
        }

        EventContextGuard(const EventContextGuard &) = delete;
        EventContextGuard &operator=(const EventContextGuard &) = delete;
    };

    /**
     * @brief Event descriptor for queued events with W3C SCXML priority support
     */
    struct QueuedEvent {
        std::string eventName;
        std::string eventData;
        std::string origin;      // §scxml-5.10.1: _event.origin - Session that originated this event
        std::string sendId;      // §scxml-5.10.1: _event.sendid - sendid from send element
        std::string invokeId;    // §scxml-5.10.1: _event.invokeid - invokeid from invoked child process
        std::string originType;  // §scxml-5.10.1: _event.origintype - event processor type
        std::chrono::steady_clock::time_point timestamp;
        EventPriority priority;
        std::optional<ScriptValue> typedData;  // Engine-agnostic typed data (avoids JSON round-trip)

        QueuedEvent(const std::string &name, const std::string &data, EventPriority prio = EventPriority::INTERNAL,
                    const std::string &originSessionId = "", const std::string &sid = "", const std::string &iid = "",
                    const std::string &otype = "",
                    std::chrono::steady_clock::time_point ts = std::chrono::steady_clock::time_point(),
                    std::optional<ScriptValue> typed = std::nullopt)
            : eventName(name), eventData(data), origin(originSessionId), sendId(sid), invokeId(iid), originType(otype),
              timestamp(ts.time_since_epoch().count() > 0 ? ts : std::chrono::steady_clock::now()), priority(prio),
              typedData(std::move(typed)) {}
    };

    /**
     * @brief Comparator for priority queue - orders by priority (INTERNAL first) then timestamp (FIFO)
     * Note: std::priority_queue is a max-heap, so we invert the comparison
     */
    struct QueuedEventComparator {
        bool operator()(const QueuedEvent &a, const QueuedEvent &b) const {
            // Invert comparison for max-heap: we want INTERNAL (0) before EXTERNAL (1)
            if (a.priority != b.priority) {
                return a.priority > b.priority;  // Lower priority value = higher actual priority
            }
            // For same priority, older timestamp should come first (FIFO)
            return a.timestamp > b.timestamp;  // Older timestamp = lower in heap
        }
    };

    /**
     * @brief Create an EventRaiser with optional callback
     * @param callback Optional event callback function
     */
    explicit EventRaiserImpl(EventCallback callback = nullptr);

    /**
     * @brief Destructor - ensures clean shutdown
     */
    ~EventRaiserImpl();

    /**
     * @brief Set the event callback function
     * @param callback Function to call when events are raised
     */
    void setEventCallback(EventCallback callback);

    /**
     * @brief Clear the event callback
     */
    void clearEventCallback();

    /**
     * @brief Shutdown the async processing (for clean destruction)
     */
    void shutdown() override;

    /**
     * @brief Set EventScheduler for delayed event polling (WASM support)
     *
     * §scxml-6.2: Enable delayed send element support by providing scheduler access.
     * Platform-specific behavior handled by PlatformEventRaiserHelper.
     *
     * @param scheduler Shared pointer to EventScheduler instance
     *
     * Note: Optional - if not set, delayed events won't be polled (WASM will miss delayed events)
     */
    void setScheduler(std::shared_ptr<IEventScheduler> scheduler);

    /**
     * @brief Get EventScheduler for scheduler mode access
     *
     * Enable parent-child scheduler mode inheritance for interactive debugging.
     * Allows parent state machine to propagate MANUAL mode to child invoke sessions.
     *
     * @return Shared pointer to EventScheduler instance, or nullptr if not set
     */
    std::shared_ptr<IEventScheduler> getScheduler() const override;

    /**
     * @brief Cancel all queued events from a specific session (§scxml-6.4.3 compliance)
     *
     * Removes all events in the synchronous queue that originated from the specified session.
     * This is required when cancelling invokes to prevent processing events from cancelled children.
     *
     * @param originSessionId Session ID whose events should be cancelled
     * @return Number of events that were cancelled
     */
    size_t cancelEventsForSession(const std::string &originSessionId) override;

    /**
     * @brief §scxml-3.12.2: `error.*` events refused because an error handler kept raising them
     *
     * See `IEventRaiser::getErrorCascadeEvents`. This raiser owns the queue the
     * chain feeds, so it is where the chain is counted and cut.
     */
    uint32_t getErrorCascadeEvents() const override;

    /**
     * @brief The most recent event `getErrorCascadeEvents()` refused, empty when none
     */
    std::string getLastErrorCascadeEvent() const override;

    /**
     * @brief §scxml-3.12.2: forget the chain, because the host has called in again
     *
     * See `IEventRaiser::resetErrorCascadeDepth`. The count of what was refused
     * is untouched — that is a fact about the past, not a mode.
     */
    void resetErrorCascadeDepth() override;

    // IEventRaiser interface
    bool raiseEvent(const std::string &eventName, const std::string &eventData) override;
    bool raiseEvent(const std::string &eventName, const std::string &eventData,
                    const std::string &originSessionId) override;
    bool raiseEvent(const std::string &eventName, const std::string &eventData, const std::string &sendId,
                    bool) override;
    bool raiseEvent(const std::string &eventName, const std::string &eventData, const std::string &originSessionId,
                    const std::string &invokeId) override;
    bool raiseEvent(const std::string &eventName, const std::string &eventData, const std::string &originSessionId,
                    const std::string &invokeId, const std::string &originType) override;
    bool raiseInternalEvent(const std::string &eventName, const std::string &eventData) override;
    bool raiseExternalEvent(const std::string &eventName, const std::string &eventData) override;
    bool isReady() const override;
    void setImmediateMode(bool immediate) override;

    /**
     * @brief Check if immediate mode is currently enabled
     * @return true if immediate mode is enabled, false otherwise
     */
    bool isImmediateModeEnabled() const override {
        return immediateMode_.load();
    }

    void processQueuedEvents() override;

    /**
     * @brief W3C SCXML compliance: Process only ONE event from the queue
     * @return true if an event was processed, false if queue is empty
     */
    bool processNextQueuedEvent() override;

    /**
     * @brief Get information about the last processed event (for time-travel debugging)
     *
     * Enable interactive visualizer to track internal events from raise actions.
     * This allows step backward to replay internal events correctly.
     *
     * @param outEventName Output parameter for event name (empty if no event processed yet)
     * @param outEventData Output parameter for event data
     * @return true if last processed event info is available, false otherwise
     */
    bool getLastProcessedEvent(std::string &outEventName, std::string &outEventData) const;

    /**
     * @brief Check if there are queued events waiting to be processed
     * @return true if queue has events, false if empty
     */
    bool hasQueuedEvents() const override;

    /**
     * @brief Check if there are INTERNAL priority events in the queue
     *
     * §scxml-3.13: Used to enforce event priority - EXTERNAL events should not
     * use immediate mode when INTERNAL events are queued, ensuring INTERNAL events
     * are processed first.
     *
     * @return true if queue has INTERNAL priority events, false otherwise
     */
    bool hasQueuedInternalEvents() const override;

    /**
     * @brief §scxml-D-mainEventLoop: Process one INTERNAL event, leaving
     *        external events queued for after the macrostep's invokes
     * @return true if an internal event was processed, false if none was queued
     */
    bool processNextInternalEvent() override;

    /**
     * @brief Get snapshot of current event queues for visualization/debugging
     *
     * Retrieves current contents of internal and external event queues
     * for use in interactive visualization and time-travel debugging.
     *
     * @param outInternal Output vector for internal queue events
     * @param outExternal Output vector for external queue events
     */
    void getEventQueues(std::vector<EventSnapshot> &outInternal,
                        std::vector<EventSnapshot> &outExternal) const override;

    /**
     * @brief Clear all queued events (for time-travel debugging reset)
     *
     * W3C SCXML: Removes all events from internal and external queues
     * to allow clean state restoration in interactive visualization.
     */
    void clearQueue();

    /**
     * @brief Internal method to raise event with specific priority (for W3C SCXML compliance)
     * @param eventName Name of the event to raise
     * @param eventData Data associated with the event
     * @param priority Event priority (INTERNAL or EXTERNAL)
     * @param originSessionId Session ID that originated this event (for finalize)
     * @param sendId Send ID from failed send element (for error events)
     * @param invokeId Invoke ID from invoked child process (test 338)
     * @return true if the event was successfully queued, false if the raiser is not ready
     */
    bool raiseEventWithPriority(const std::string &eventName, const std::string &eventData, EventPriority priority,
                                const std::string &originSessionId = "", const std::string &sendId = "",
                                const std::string &invokeId = "", const std::string &originType = "",
                                int64_t timestampNs = 0, std::optional<ScriptValue> typedData = std::nullopt);

private:
    /**
     * @brief Background worker thread for processing events
     */
    void eventProcessingWorker();

    /**
     * @brief Process a single event from the queue
     */
    void processEvent(const QueuedEvent &event);

    /**
     * @brief Execute callback for a queued event (synchronous processing)
     *
     * §scxml-3.13: Event processing result indicates whether state transition occurred.
     * The return value reflects the callback's transition success, not just execution status.
     *
     * @param event Event to process
     * @return true if event caused successful state transition, false otherwise
     */
    bool executeEventCallback(const QueuedEvent &event);

    // Event callback
    EventCallback eventCallback_;

    // §scxml-5.10: Consolidated thread-local event context for callback execution
    static thread_local EventContext currentEventContext_;

public:
    /**
     * @brief Get the current thread-local event context
     * Set during event callback execution, contains all §scxml-5.10 event metadata.
     * StateMachine reads this during processEvent() to populate _event fields.
     */
    static const EventContext &getCurrentEventContext() {
        return currentEventContext_;
    }

    // Convenience accessors (delegate to EventContext)
    static const std::string &getCurrentOriginSessionId() {
        return currentEventContext_.originSessionId;
    }

    static const std::string &getCurrentSendId() {
        return currentEventContext_.sendId;
    }

    static const std::string &getCurrentInvokeId() {
        return currentEventContext_.invokeId;
    }

    static const std::string &getCurrentOriginType() {
        return currentEventContext_.originType;
    }

    static const std::string &getCurrentEventType() {
        return currentEventContext_.eventType;
    }

    /**
     * @brief Whether the event being dispatched came off the external queue
     *
     * W3C SCXML Appendix D mainEventLoop performs autoforward and
     * `<finalize>` at one point only: after `externalQueue.dequeue()`.
     * Internal-queue events never reach that point, which is what keeps
     * `error.*` and `done.state.*` out of the forwarded set — the exclusion
     * is the queue they were raised onto, not how their names are spelled.
     */
    static bool isCurrentEventFromExternalQueue() {
        return currentEventContext_.isExternalQueue;
    }

    static const std::optional<ScriptValue> &getCurrentTypedData() {
        return currentEventContext_.typedData;
    }

    mutable std::mutex callbackMutex_;

    // Platform-specific event processing helper (Zero Duplication Principle)
    std::unique_ptr<PlatformEventRaiserHelper> platformHelper_;

    // §scxml-6.2: EventScheduler for delayed event polling (WASM support)
    std::shared_ptr<IEventScheduler> scheduler_;

    // Asynchronous processing infrastructure
    std::queue<QueuedEvent> eventQueue_;
    std::mutex queueMutex_;
    std::condition_variable queueCondition_;
    std::atomic<bool> shutdownRequested_;
    std::atomic<bool> isRunning_;

    // SCXML compliance mode and synchronous queue
    std::atomic<bool> immediateMode_;
    std::priority_queue<QueuedEvent, std::vector<QueuedEvent>, QueuedEventComparator> synchronousQueue_;
    mutable std::mutex synchronousQueueMutex_;

    // Time-travel debugging support - track last processed event
    std::string lastProcessedEventName_;
    std::string lastProcessedEventData_;
    mutable std::mutex lastProcessedEventMutex_;

    // §scxml-3.12.2: this raiser is dispatching an `error.*` event, which is
    // the state in which a newly raised error is a link in a chain rather than
    // a first failure; how long that chain is; and what was refused because of
    // it. See `getErrorCascadeEvents()`.
    //
    // Dispatch here is re-entrant — executable content runs a nested drain —
    // so the flag is saved and restored around each dispatch rather than
    // cleared, and every field is atomic because the async worker thread
    // dispatches through the same path.
    std::atomic<bool> handlingErrorEvent_{false};
    // How many dispatches are on the stack. The five queue-draining engines
    // end a chain when the internal queue empties; this raiser has no such
    // moment — executable content dispatches into it again — so the equivalent
    // is the OUTERMOST dispatch returning, which is what this counts.
    std::atomic<uint32_t> dispatchDepth_{0};
    std::atomic<uint32_t> errorCascadeDepth_{0};
    std::atomic<uint32_t> errorCascadeEvents_{0};
    std::string lastErrorCascadeEvent_;
    mutable std::mutex lastErrorCascadeEventMutex_;

    /**
     * @brief Marks one dispatch as "an error handler is running" for the raise
     *        path to read, and ends the chain when the dispatch is anything else
     *
     * Restores the previous value rather than clearing it: a transition's
     * executable content dispatches through this raiser again, so the scopes
     * nest, and clearing would tell the inner frame's raise that it is a first
     * failure.
     */
    struct ErrorChainScope {
        ErrorChainScope(EventRaiserImpl &raiser, const std::string &eventName);
        ~ErrorChainScope();

        ErrorChainScope(const ErrorChainScope &) = delete;
        ErrorChainScope &operator=(const ErrorChainScope &) = delete;

    private:
        EventRaiserImpl &raiser_;
        bool previous_;
    };
};

}  // namespace SCE