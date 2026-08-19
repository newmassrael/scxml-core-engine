// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <memory>
#include <string>
#include <vector>

namespace SCE {

// Forward declarations
struct EventSnapshot;

/**
 * @brief Interface for raising events in the SCXML system
 *
 * This interface implements the SCXML "fire and forget" event model as specified
 * by W3C SCXML standard. Events are processed asynchronously to prevent deadlocks
 * and ensure proper event ordering. The interface separates event raising from
 * action execution, following the Single Responsibility Principle.
 */
class IEventRaiser {
public:
    virtual ~IEventRaiser() = default;

    /**
     * @brief Raise an event with the given name and data (SCXML "fire and forget")
     *
     * Events are queued for asynchronous processing and this method returns immediately.
     * This implements the SCXML "fire and forget" model to prevent deadlocks and ensure
     * proper event ordering as specified by W3C SCXML standard.
     *
     * @param eventName Name of the event to raise
     * @param eventData Data associated with the event
     * @return true if the event was successfully queued, false if the raiser is not ready
     */
    virtual bool raiseEvent(const std::string &eventName, const std::string &eventData) = 0;

    /**
     * @brief Raise an event with origin tracking for W3C SCXML finalize support
     *
     * Events are queued for asynchronous processing with origin session information.
     * This enables proper finalize handler execution as specified by §scxml-6.4.
     *
     * @param eventName Name of the event to raise
     * @param eventData Data associated with the event
     * @param originSessionId Session ID that originated this event (for finalize)
     * @return true if the event was successfully queued, false if the raiser is not ready
     */
    virtual bool raiseEvent(const std::string &eventName, const std::string &eventData,
                            const std::string &originSessionId) = 0;

    /**
     * @brief Raise an error event with sendid for §scxml-5.10 compliance
     *
     * When send actions fail, error events must include the sendid of the failed send element.
     * This enables test 332 compliance where error.execution event must contain sendid.
     *
     * @param eventName Name of the event to raise (typically "error.execution")
     * @param eventData Data associated with the event
     * @param sendId Send ID from the failed send element
     * @param unused Discriminator parameter for overload resolution (unused, always pass false)
     * @return true if the event was successfully queued, false if the raiser is not ready
     *
     * @note The bool parameter exists solely for C++ overload resolution to distinguish
     *       this variant from raiseEvent(name, data, originSessionId). Both take three
     *       string parameters, requiring a discriminator to avoid ambiguity.
     */
    virtual bool raiseEvent(const std::string &eventName, const std::string &eventData, const std::string &sendId,
                            bool unused) = 0;

    /**
     * @brief Raise an event with origin and invoke tracking for §scxml-5.10 test 338
     *
     * Events from invoked children are queued with both origin and invoke ID information.
     * This enables proper event.invokeid field setting as specified by §scxml-5.10.
     *
     * @param eventName Name of the event to raise
     * @param eventData Data associated with the event
     * @param originSessionId Session ID that originated this event (for finalize)
     * @param invokeId Invoke ID that created the child session (for event.invokeid)
     * @return true if the event was successfully queued, false if the raiser is not ready
     */
    virtual bool raiseEvent(const std::string &eventName, const std::string &eventData,
                            const std::string &originSessionId, const std::string &invokeId) = 0;

    /**
     * @brief Raise an event with origin, invoke, and origintype for §scxml-5.10 compliance
     *
     * Events are queued with origin, invoke ID, and origintype information for full W3C compliance.
     * This enables proper event metadata (test 253, 331, 352, 372: origintype field).
     *
     * @param eventName Name of the event to raise
     * @param eventData Data associated with the event
     * @param originSessionId Session ID that originated this event (for finalize)
     * @param invokeId Invoke ID that created the child session (for event.invokeid)
     * @param originType Origin event processor type (for event.origintype)
     * @return true if the event was successfully queued, false if the raiser is not ready
     */
    virtual bool raiseEvent(const std::string &eventName, const std::string &eventData,
                            const std::string &originSessionId, const std::string &invokeId,
                            const std::string &originType) = 0;

    virtual bool isReady() const = 0;

    /**
     * @brief Shut down the event raiser and release all resources
     *
     * Stops the worker thread, clears event queues, and releases the scheduler.
     * Safe to call multiple times. After shutdown, raiseEvent calls will fail.
     */
    virtual void shutdown() = 0;

    /**
     * @brief Set execution mode for SCXML compliance
     * @param immediate true for immediate processing, false for queued processing
     */
    virtual void setImmediateMode(bool immediate) = 0;

    /**
     * @brief Check if immediate mode is currently enabled
     * @return true if immediate mode is enabled, false otherwise
     */
    virtual bool isImmediateModeEnabled() const = 0;

    /**
     * @brief Process all queued events synchronously (for SCXML compliance)
     * This method processes queued events in order and returns when all are processed
     */
    virtual void processQueuedEvents() = 0;

    /**
     * @brief W3C SCXML compliance: Process only ONE event from the queue
     * @return true if an event was processed, false if queue is empty
     */
    virtual bool processNextQueuedEvent() = 0;

    /**
     * @brief Check if there are queued events waiting to be processed
     * @return true if queue has events, false if empty
     */
    virtual bool hasQueuedEvents() const = 0;

    /**
     * @brief §scxml-D-mainEventLoop: Process only ONE *internal* event, leaving
     *        external events queued
     *
     * The macrostep completes on eventless transitions and internal events
     * alone; `invoke(inv)` then runs for the states it entered, and only after
     * that does the algorithm reach `externalQueue.dequeue()`. A drain that
     * cannot tell the two classes apart consumes an external event while the
     * invokes are still pending, and an `autoforward` child never sees it.
     *
     * @return true if an internal event was processed, false if none was queued
     */
    virtual bool processNextInternalEvent() = 0;

    /**
     * @brief §scxml-3.13: Check whether an INTERNAL-priority event is queued
     * @return true if the queue holds an internal event, false otherwise
     */
    virtual bool hasQueuedInternalEvents() const = 0;

    /**
     * @brief Get snapshot of current event queues for visualization/debugging
     *
     * Retrieves current contents of internal and external event queues
     * for use in interactive visualization and time-travel debugging.
     *
     * @param outInternal Output vector for internal queue events
     * @param outExternal Output vector for external queue events
     */
    virtual void getEventQueues(std::vector<struct EventSnapshot> &outInternal,
                                std::vector<struct EventSnapshot> &outExternal) const = 0;

    /**
     * @brief Raise an internal event (§scxml-3.13: higher priority than external events)
     *
     * Internal events are raised by <raise> elements and have higher priority than
     * external events. This ensures proper event queue ordering as specified by W3C SCXML.
     *
     * @param eventName Name of the event to raise
     * @param eventData Data associated with the event
     * @return true if the event was successfully queued, false if the raiser is not ready
     */
    virtual bool raiseInternalEvent(const std::string &eventName, const std::string &eventData) = 0;

    /**
     * @brief Raise an external event (§scxml-5.10: lower priority than internal events)
     *
     * External events come from external I/O processors (HTTP, WebSocket, etc.) and have
     * lower priority than internal events. This ensures proper event queue ordering for
     * W3C SCXML compliance (test 510).
     *
     * @param eventName Name of the event to raise
     * @param eventData Data associated with the event
     * @return true if the event was successfully queued, false if the raiser is not ready
     */
    virtual bool raiseExternalEvent(const std::string &eventName, const std::string &eventData) = 0;

    /**
     * @brief Get EventScheduler for scheduler mode access
     *
     * Enable parent-child scheduler mode inheritance for interactive debugging.
     * Allows parent state machine to propagate MANUAL mode to child invoke sessions.
     *
     * @return Shared pointer to EventScheduler instance, or nullptr if not set
     */
    virtual std::shared_ptr<class IEventScheduler> getScheduler() const = 0;

    /**
     * @brief Cancel all queued events from a specific session (§scxml-6.4.3 compliance)
     *
     * §scxml-6.4.3: "Once it cancels the invoked session, the Processor MUST ignore any
     * events it receives from that session. In particular it MUST NOT insert them into
     * the external event queue of the invoking session"
     *
     * This method removes all queued events that originated from the specified session.
     * Used when cancelling invokes to prevent processing events from cancelled child sessions.
     *
     * @param originSessionId Session ID whose events should be cancelled
     * @return Number of events that were cancelled
     */
    virtual size_t cancelEventsForSession(const std::string &originSessionId) = 0;

    /**
     * @brief §scxml-3.12.2: `error.*` events refused because an error handler kept raising them
     *
     * The clause says an error event nothing matches is ignored. It says nothing
     * about one that IS matched by a handler that fails the same way every time:
     * the failure raises the error, the same transition answers it, and the
     * processing never comes back to its caller. Nothing in the specification
     * bounds that, so the raiser that owns the queue is the party that has to.
     *
     * Defaulted rather than pure: a raiser that never refuses anything — every
     * test double here is one — answers zero truthfully, and making this pure
     * would put the same `return 0;` in each of them.
     *
     * @return Count of refused error events, zero when none were
     */
    virtual uint32_t getErrorCascadeEvents() const {
        return 0;
    }

    /**
     * @brief The most recent event `getErrorCascadeEvents()` refused
     *
     * Empty while that count is zero. Which error it was names the repair:
     * `error.execution` is a handler whose own executable content fails,
     * `error.communication` one that answers an unreachable target by talking
     * to it again.
     *
     * @return Name of the last refused error event, empty when there is none
     */
    virtual std::string getLastErrorCascadeEvent() const {
        return {};
    }

    /**
     * @brief §scxml-3.12.2: a new piece of host work begins, so any error chain is over
     *
     * The queue-draining engines reset this as the internal queue empties. A
     * raiser whose dispatches are serialized through the state machine's own
     * entry point has no such moment — every dispatch looks like the outermost
     * one — so the boundary is the host's call. Refusing to reset at all would
     * make the ceiling a property of the machine's whole life rather than of
     * one chain.
     *
     * Defaulted to nothing: a raiser that never counts a chain has none to
     * forget.
     */
    virtual void resetErrorCascadeDepth() {}
};

}  // namespace SCE