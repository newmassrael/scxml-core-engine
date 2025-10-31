#pragma once

#include "ScheduledAotTest.h"
#include "test207_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.3: cancel tag - cannot cancel events in different sessions
 *
 * This test verifies that a parent session cannot cancel delayed send events
 * that were raised in a child invoked session. The test invokes a child process
 * that sends a delayed event with sendid 'foo' and notifies the parent. The parent
 * attempts to cancel the event using <cancel sendid="foo"/>, but this should fail
 * because the sendid is scoped to the child session only.
 *
 * W3C SCXML Specification Compliance:
 * - W3C SCXML 6.4: invoke with inline <content> (child state machine instantiation)
 * - W3C SCXML 6.2: send with delay attribute (delayed event scheduling)
 * - W3C SCXML 6.3: cancel with sendid attribute (event cancellation)
 * - W3C SCXML 5.10.1: #_parent target (parent-child communication)
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 * - Fully static state machine (compile-time states/transitions)
 * - No JSEngine needed (all event names, delays, targets are static literals)
 * - Uses Helper functions:
 *   - SendSchedulingHelper: Delayed send event scheduling
 *   - SendHelper: Event delivery with target resolution (#_parent)
 *   - InvokeHelper: Child session management with inline content
 *   - EventTargetFactory: SCXML event target resolution
 *
 * Key W3C SCXML Features:
 * - Session-scoped sendid: sendid namespace isolated per session
 * - Cross-session communication: Parent-child event delivery via #_parent
 * - Event scheduler: Delayed send with timeout and cancellation
 * - Invoke lifecycle: Child process creation, event routing, termination
 *
 * Expected Behavior (W3C SCXML 6.3):
 * - Child raises delayed event1 (1s delay) with sendid="foo"
 * - Child raises delayed event2 (1.5s delay) as fallback
 * - Child notifies parent via childToParent event
 * - Parent attempts <cancel sendid="foo"/> (should fail - different session)
 * - event1 fires successfully in child → child sends "pass" to parent
 * - If event1 was canceled, event2 fires → child sends "fail" to parent
 */
struct Test207 : public ScheduledAotTest<Test207, 207> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.3: cancel operation scoped to originating session, cannot affect child session's delayed events";
    using SM = RSM::Generated::test207::test207;
};

// Auto-register
inline static AotTestRegistrar<Test207> registrar_Test207;

}  // namespace RSM::W3C::AotTests
