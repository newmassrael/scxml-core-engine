#pragma once

#include "SimpleAotTest.h"
#include "test199_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.2: Unsupported send type raises error.execution
 *
 * Tests that using an unsupported event I/O processor type in <send> element
 * raises error.execution event and places it on internal event queue.
 *
 * W3C SCXML 6.2 Specification:
 * If the SCXML Processor does not support the type that is specified in <send>,
 * it MUST place the event error.execution on the internal event queue.
 *
 * Test Flow:
 * 1. State s0 enters and executes entry actions
 * 2. <send type="unsupported_type" event="event1"/> triggers error.execution
 * 3. <send event="timeout"/> queues timeout to external queue
 * 4. error.execution (internal queue) has higher priority than timeout (external queue)
 * 5. Transition on error.execution fires → pass state (test success)
 * 6. Wildcard transition would fire on any other event → fail state (test failure)
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 * - Fully static state machine (compile-time states/transitions)
 * - No JSEngine needed (no ECMAScript expressions)
 * - Uses SendHelper for send type validation
 * - Zero runtime overhead (all logic compile-time generated)
 *
 * W3C SCXML Features:
 * - 6.2: Send element with type attribute
 * - 5.10.1: Internal vs external event queue priority
 * - 5.10: Error event generation (error.execution)
 * - 3.13: Wildcard event matching in transitions
 */
struct Test199 : public SimpleAotTest<Test199, 199> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.2: unsupported send type raises error.execution (Pure Static AOT)";
    using SM = RSM::Generated::test199::test199;
};

// Auto-register
inline static AotTestRegistrar<Test199> registrar_Test199;

}  // namespace RSM::W3C::AotTests
