#pragma once
#include "SimpleAotTest.h"
#include "test553_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.2.4 & 5.11: Send namelist error handling
 *
 * Verifies that the processor does NOT dispatch an event when evaluation
 * of <send> namelist attribute fails (variable not found in datamodel).
 * According to W3C SCXML 6.2.4 and 5.11, if namelist evaluation produces
 * an error, the processor MUST discard the message and raise error.execution.
 *
 * Test flow:
 * 1. State machine starts in s0
 * 2. s0 onentry schedules two sends:
 *    a. timeout event with 1s delay (will arrive if event1 is not sent)
 *    b. event1 with namelist="__undefined_variable_for_error__" (immediate send)
 * 3. Namelist evaluation fails because __undefined_variable_for_error__ is undefined
 * 4. event1 send is discarded, error.execution is raised (test ignores this)
 * 5. After 1s, timeout event arrives → transition to pass
 * 6. If event1 arrives instead, test fails (meaning namelist error was not handled)
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and namelist variable validation
 * - Uses Helper functions: NamelistHelper (for namelist evaluation and error handling)
 * - Event scheduler polling for delayed send (W3C SCXML 6.2)
 *
 * W3C SCXML Features:
 * - Send namelist attribute (C.1)
 * - Namelist error handling (6.2.4 & 5.11)
 * - ECMAScript datamodel variable lookup (B.2)
 * - Delayed send with event scheduling (6.2)
 * - Error.execution event (5.10)
 *
 * Implementation Details:
 * - NamelistHelper::evaluateNamelist() uses JSEngine.getVariable() to check variable existence
 * - Returns false if any namelist variable is undefined
 * - Early return prevents event dispatch when namelist validation fails
 * - ScheduledAotTest polls event scheduler for delayed timeout event (1s)
 * - Static Hybrid: Static structure + JSEngine for runtime variable checking
 */
struct Test553 : public ScheduledAotTest<Test553, 553> {
    static constexpr const char *DESCRIPTION = "Send namelist error handling (W3C 6.2.4 AOT Static Hybrid)";
    using SM = RSM::Generated::test553::test553;
};

// Auto-register
inline static AotTestRegistrar<Test553> registrar_Test553;

}  // namespace RSM::W3C::AotTests
