// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "core/EventQueueConcept.h"
#include "core/LogMacros.h"
#include <functional>

namespace SCE::Core {

/**
 * @brief W3C SCXML event processing algorithms (Single Source of Truth)
 *
 * Share all event processing logic for Interpreter and AOT engines based on templates.
 *
 * Design principles:
 * 1. Share algorithms only, maintain per-engine data structure optimization
 * 2. Template-based zero overhead (inline expansion)
 * 3. Ensure type safety with clear interfaces
 *
 * @note All methods in this class are static template functions,
 *       inlined at compile time with no runtime overhead.
 */
/**
 * @brief The default answer to "may this macrostep take another microstep?".
 *
 * A named type rather than a lambda default so the parameter can carry a
 * default template argument: yes, forever, which is what the queue drain did
 * before any engine here bounded it.
 */
struct AlwaysTakeMicrostep {
    bool operator()() const {
        return true;
    }
};

class EventProcessingAlgorithms {
public:
    /**
     * @brief Process the internal event queue (FIFO)
     *
     * Exhaust all internal events in FIFO order when macrostep completes.
     * Both Interpreter and AOT engines use the same algorithm.
     *
     * @tparam EventQueue Event queue type
     *   Required methods: bool hasEvents() const, EventType popNext()
     * @tparam EventHandler Event handler callback type
     *   Signature: bool handler(EventType event)
     *
     * @param queue Internal event queue (AOTEventQueue or InterpreterEventQueue)
     * @param handler Event processing function (stops processing if returns false)
     * @param mayTakeMicrostep Asked before each dequeue: may this macrostep
     *        take another microstep? A drain that stops here leaves the event
     *        on the queue, which is the difference between a chain this engine
     *        declined to keep running and one it silently swallowed. The
     *        default answers yes forever, which is what every caller did
     *        before a `<raise>` that answers itself was measured to be a
     *        macrostep that never ends. A caller that says no is expected to
     *        publish the refusal there — the loop cannot, because it does not
     *        know whose ceiling it is.
     *
     * @example AOT engine:
     * @code
     * AOTEventQueue aotQueue(eventQueue_);
     * processInternalEventQueue(aotQueue, [this](Event e) {
     *     return processInternalEvent(e);
     * });
     * @endcode
     *
     * @example Interpreter engine:
     * @code
     * InterpreterEventQueue interpQueue(eventRaiser_);
     * processInternalEventQueue(interpQueue, [this](auto) {
     *     return true;  // EventRaiser handles internally
     * });
     * @endcode
     */
#if __cpp_concepts >= 202002L
    template <EventQueueAdapter EventQueue, typename EventHandler, typename MicrostepBudget = AlwaysTakeMicrostep>
#else
    template <typename EventQueue, typename EventHandler, typename MicrostepBudget = AlwaysTakeMicrostep>
#endif
    static void processInternalEventQueue(EventQueue &queue, EventHandler &&handler,
                                          MicrostepBudget &&mayTakeMicrostep = MicrostepBudget{}) {
        // §scxml-3.13: Process all internal events in FIFO order
        while (queue.hasEvents()) {
            if (!mayTakeMicrostep()) {
                // The macrostep ran out of budget with work still queued. The
                // event is deliberately left where it is: the next macrostep
                // starts there, and nothing the document raised is lost.
                SCE_LOG_DEBUG("EventProcessingAlgorithms: microstep budget spent, leaving the queue for the next "
                              "macrostep");
                break;
            }
            auto event = queue.popNext();

            // Stop if event processing fails
            if (!handler(event)) {
                SCE_LOG_DEBUG("EventProcessingAlgorithms: Event handler returned false, stopping queue processing");
                break;
            }
        }
    }

    // Two more templates stood here, `checkEventlessTransitions` and
    // `processMacrostep`, presented as the macrostep both engines share. No
    // engine called either, and neither was Appendix D's loop: they drained the
    // internal queue BEFORE eventless transitions, and moved the machine by
    // running onexit for one "current" state and onentry for another — a
    // chain's worth of states and a `<parallel>`'s regions never entered the
    // picture. The microstep both engines share is `MicrostepAlgorithms`; each
    // engine's main event loop drives it. An unreachable copy of an appendix
    // procedure drifts unseen, so it is gone rather than kept as a reference.
};

}  // namespace SCE::Core
