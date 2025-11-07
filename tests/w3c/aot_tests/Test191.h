#pragma once
#include "ScheduledAotTest.h"
#include "test191_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4.1: Inline <content> invoke with #_parent target
 *
 * Tests that a parent SCXML session can invoke a child session using inline <content>,
 * and the child can send events to the parent using #_parent as the target.
 *
 * Test Scenario:
 * 1. Parent state s0 enters and:
 *    - Sends timeout event with 5s delay (fallback to prevent hanging)
 *    - Invokes child SCXML session via <invoke type="scxml"> with inline <content>
 * 2. Child session (test191_child0):
 *    - State sub0 enters and sends "childToParent" event to #_parent target
 *    - Transitions to subFinal (terminates child session)
 * 3. Parent receives "childToParent" event from child → transitions to pass
 * 4. If timeout occurs first → transitions to fail
 *
 * Uses ScheduledAotTest for runUntilCompletion() to process:
 * - Deferred static invoke execution (W3C SCXML 6.4)
 * - AOT child state machine lifecycle
 * - Event scheduler polling for timeout and child events
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 * - Fully static state machine (compile-time states/transitions)
 * - No JSEngine needed (datamodel declared but unused)
 * - Child SCXML from inline <content> compiled to separate test191_child0 state machine
 * - Uses Helper functions: SendHelper for #_parent routing, InvokeHelper for child lifecycle
 * - W3C SCXML 3.12.1: Automatic invoke ID generation in "stateid.platformid.index" format
 *   (index suffix ensures uniqueness for multiple invokes in same state)
 *
 * W3C SCXML Features:
 * - W3C SCXML 6.4.1: <invoke type="scxml"> with inline <content> element
 * - W3C SCXML 6.4.5: #_parent target for parent-child communication
 * - W3C SCXML 3.14: SCXML sessions (parent-child relationship)
 * - W3C SCXML 6.2: <send> with target attribute
 * - W3C SCXML 6.2.5: Delayed send for timeout handling
 */
struct Test191 : public ScheduledAotTest<Test191, 191> {
    static constexpr const char *DESCRIPTION = "Inline content invoke with #_parent (W3C 6.4.1 AOT Pure Static)";
    using SM = SCE::Generated::test191::test191;
};

// Auto-register
inline static AotTestRegistrar<Test191> registrar_Test191;

}  // namespace SCE::W3C::AotTests
