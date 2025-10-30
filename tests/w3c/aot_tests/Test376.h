#pragma once
#include "SimpleAotTest.h"
#include "test376_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 3.8: Independent onentry handler execution
 *
 * Tests that each <onentry> handler is a separate block that executes independently.
 * Even if one onentry handler throws an error (invalid send target), subsequent
 * onentry handlers should continue executing.
 *
 * Test Structure:
 * - State s0 has TWO onentry handlers:
 *   1. First onentry: <send target="!invalid" event="event1"/> → triggers error.execution
 *   2. Second onentry: <assign location="Var1" expr="Var1 + 1"/> → should execute anyway
 * - Transition with cond="Var1 == 2" verifies second onentry executed
 *
 * Expected Behavior (W3C SCXML 3.8):
 * - First onentry handler fails (invalid send target "!invalid")
 * - error.execution event raised but does NOT stop state entry processing
 * - Second onentry handler MUST execute (Var1 incremented from 1 to 2)
 * - Eventless transition evaluates: Var1 == 2 → true → transition to pass
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * Static Components:
 * - State machine structure (compile-time State enum: s0, pass, fail)
 * - Transition logic (switch-case for state transitions)
 * - Event types (compile-time Event enum)
 * - Entry action structure (two separate onentry blocks)
 *
 * Dynamic Components (JSEngine):
 * - ECMAScript datamodel: Var1 variable storage
 * - Expression evaluation: "Var1 + 1" (assign expression)
 * - Condition evaluation: "Var1 == 2" (transition guard)
 * - Error handling: error.execution event generation
 *
 * Helper Functions Used (ARCHITECTURE.md Zero Duplication):
 * - DatamodelHelper: Initialize ECMAScript datamodel with Var1 = 1
 * - AssignHelper: Execute assign action (Var1 = Var1 + 1)
 * - GuardHelper: Evaluate transition condition (Var1 == 2)
 * - SendHelper: Detect invalid send target and raise error.execution
 * - EventMetadataHelper: Bind error.execution event metadata
 *
 * W3C SCXML Features:
 * - 3.8: Multiple onentry handlers with independent execution
 * - 5.9.1: Assign action with ECMAScript expression
 * - 6.2.1: Send action with target validation
 * - 3.12.1: error.execution event on send failure
 * - 5.9: Transition conditions with ECMAScript expressions
 */
struct Test376 : public SimpleAotTest<Test376, 376> {
    static constexpr const char *DESCRIPTION = "Independent onentry handler execution (W3C 3.8 AOT Static Hybrid)";
    using SM = RSM::Generated::test376::test376;
};

// Auto-register
inline static AotTestRegistrar<Test376> registrar_Test376;

}  // namespace RSM::W3C::AotTests
