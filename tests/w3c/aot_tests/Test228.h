#pragma once

#include "ScheduledAotTest.h"
#include "test228_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.3.1: Invoke ID in done.invoke event
 *
 * Tests that when an invoked child state machine completes, the done.invoke event
 * contains the invoke ID in the _event.invokeid field. The parent state machine
 * assigns this value to Var1 and then verifies Var1 == 'foo' (the invoke ID).
 *
 * Uses ScheduledAotTest for runUntilCompletion() to process:
 * - Deferred static invoke execution (W3C SCXML 6.4)
 * - AOT child state machine lifecycle (test228_child0 - Pure Static)
 * - Event scheduler polling for timeout and child completion
 *
 * W3C SCXML Features:
 * - 6.3.1 (Invoke Element): Static invoke ID specification
 * - 6.4 (Invoke): Inline content child with SCXML type
 * - 3.12.1 (Invoke ID): Automatic ID generation in "stateid.platformid.index" format
 *   (index suffix ensures uniqueness for multiple invokes in same state)
 * - 5.9.1 (Done Event): done.invoke.foo event with invokeid in _event.invokeid
 *
 * Implementation Strategy:
 * - Static Hybrid: Parent uses JSEngine for ECMAScript expressions (_event.invokeid, Var1 == 'foo')
 * - Pure Static: Child (test228_child0) is simple final state
 * - All-or-Nothing: Parent is AOT Static Hybrid, child is AOT Pure Static (no Interpreter mixing)
 */
struct Test228 : public ScheduledAotTest<Test228, 228> {
    static constexpr const char *DESCRIPTION = "Invoke ID in done.invoke event (W3C 6.3.1 AOT Static Hybrid)";
    using SM = RSM::Generated::test228::test228;
};

// Auto-register with AotTestRegistry
inline static AotTestRegistrar<Test228> registrar_Test228;

}  // namespace RSM::W3C::AotTests
