#pragma once

#include "ScheduledAotTest.h"
#include "test210_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.3: sendidexpr for dynamic cancel expression evaluation
 *
 * This test verifies that sendidexpr attribute works correctly with cancel tag.
 * The test sends a delayed event1 with id="foo", then updates variable Var1 to "foo",
 * and uses <cancel sendidexpr="Var1"/> to cancel the event using the variable's value.
 *
 * Test Flow:
 * 1. Initialize Var1 = "bar"
 * 2. Send event1 with delay=1s and id="foo"
 * 3. Send event2 with delay=1.5s (fallback event)
 * 4. Assign Var1 = "foo" (now matches event1's sendid)
 * 5. Cancel using sendidexpr="Var1" (evaluates to "foo")
 * 6. If cancel works: event1 canceled, event2 arrives first → PASS
 * 7. If cancel fails: event1 arrives first → FAIL
 *
 * W3C SCXML Specification Compliance:
 * - W3C SCXML 6.3: cancel with sendidexpr attribute (dynamic sendid evaluation)
 * - W3C SCXML 6.2: send with delay and id attributes (delayed event scheduling)
 * - W3C SCXML 5.9.2: assign with expr attribute (variable assignment)
 * - W3C SCXML 5.9.1: datamodel initialization with expr
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 * - Static state machine structure (compile-time states: s0, pass, fail)
 * - JSEngine for ECMAScript datamodel and expression evaluation
 * - Uses Helper functions:
 *   - SendSchedulingHelper: Delayed send event scheduling with sendid
 *   - CancelHelper: Event cancellation with sendidexpr evaluation
 *   - AssignHelper: Variable assignment with JSEngine integration
 *   - DatamodelHelper: Variable initialization and access
 *
 * Key W3C SCXML Features:
 * - sendidexpr: Dynamic sendid evaluation using ECMAScript expressions
 * - Variable binding: Var1 value changes affect cancel target
 * - Event scheduling: Delayed send with cancellation support
 * - Expression evaluation: JSEngine evaluates expr and sendidexpr at runtime
 */
struct Test210 : public ScheduledAotTest<Test210, 210> {
    static constexpr const char *DESCRIPTION = "sendidexpr dynamic cancel (W3C 6.3 Static Hybrid)";
    using SM = SCE::Generated::test210::test210;
};

// Auto-register
inline static AotTestRegistrar<Test210> registrar_Test210;

}  // namespace SCE::W3C::AotTests
