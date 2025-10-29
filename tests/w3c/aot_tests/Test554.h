#pragma once
#include "ScheduledAotTest.h"
#include "test554_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4 & B.1: Invoke with invalid namelist error handling
 *
 * Verifies that invoke element with invalid namelist parameter causes invocation
 * cancellation (error.execution event), preventing done.invoke event before timer expires.
 * According to W3C SCXML B.1, if invoke namelist evaluation produces an error,
 * the processor MUST cancel the invocation and raise error.execution.
 *
 * Test flow:
 * 1. State machine starts in s0
 * 2. s0 onentry schedules timer event (1s delay)
 * 3. s0 invoke deferred with namelist="__undefined_variable_for_error__"
 * 4. At macrostep end, attempt to execute pending invoke
 * 5. Namelist evaluation fails → cancel invocation, raise error.execution
 * 6. Timer fires (1s) → transition to pass
 * 7. If done.invoke arrives instead → transition to fail (namelist error not handled)
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 *
 * - Fully static state machine (compile-time states/transitions)
 * - No JSEngine needed (all values are static literals)
 * - Uses Helper functions: InvokeHelper (for invoke lifecycle management)
 * - Static child SCXML: test554_child0.scxml generated at compile-time
 * - Event scheduler polling for delayed timer event (1s)
 *
 * W3C SCXML Features:
 * - Invoke element with content (6.4)
 * - Invoke namelist parameter (B.1)
 * - Invoke error handling (6.4 & B.1)
 * - error.execution event (5.10)
 * - Delayed send with event scheduling (6.2)
 * - done.invoke event (6.4)
 *
 * Implementation Details:
 * - InvokeHelper validates namelist variables before child invocation
 * - Returns false if any namelist variable is undefined
 * - Invocation cancelled when namelist validation fails
 * - ScheduledAotTest polls event scheduler for delayed timer event (1s)
 * - Pure Static: All invoke parameters are static literals (no runtime evaluation)
 */
struct Test554 : public ScheduledAotTest<Test554, 554> {
    static constexpr const char *DESCRIPTION = "Invoke namelist error handling (W3C 6.4 & B.1 AOT Pure Static)";
    using SM = RSM::Generated::test554::test554;
};

// Auto-register
inline static AotTestRegistrar<Test554> registrar_Test554;

}  // namespace RSM::W3C::AotTests
