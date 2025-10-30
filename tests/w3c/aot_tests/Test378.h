#pragma once
#include "SimpleAotTest.h"
#include "test378_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 3.8/3.9: Independent onexit handler execution with error.execution
 *
 * Test Description:
 * - Verifies that each onexit handler is executed as an independent block
 * - State s0 has two onexit handlers:
 *   1. First handler: <send target="!invalid" event="event1"/> (raises error.execution)
 *   2. Second handler: <assign location="Var1" expr="Var1 + 1"/> (increments Var1)
 * - The error in the first handler MUST NOT prevent the second handler from executing
 * - State s1 checks if Var1 == 2 (initial value 1 + increment 1 = 2) → pass
 * - If Var1 != 2 (second handler didn't execute) → fail
 *
 * W3C SCXML 3.8 Specification:
 * - Exit handlers execute when a state is exited during a transition
 * - Multiple exit handlers are independent blocks
 * - Error in one exit handler does NOT stop subsequent exit handlers
 * - All exit handlers complete before entering new states
 *
 * W3C SCXML 3.9 Specification:
 * - Executable content errors (like invalid send target) raise error.execution
 * - error.execution event is placed on internal queue
 * - Subsequent executable content in the SAME block stops
 * - Subsequent blocks (different onexit handlers) continue executing
 *
 * W3C SCXML 5.10 Specification:
 * - error.execution event generated when <send> has invalid target
 * - Invalid target examples: "!invalid" (! prefix is invalid)
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * Static State Machine Structure:
 * - Compile-time states: s0, s1, pass, fail (State enum)
 * - Compile-time events: event1, error.execution (Event enum)
 * - Compile-time transitions: s0→s1, s1→pass/fail based on Var1
 * - JSEngine needed for ECMAScript datamodel and expression evaluation
 *
 * Dynamic Runtime Elements:
 * - ECMAScript datamodel: Var1 (initialized to 1, incremented to 2)
 * - Expression evaluation: "Var1 + 1" in assign, "Var1 == 2" in condition
 * - Variable assignment: Var1 = Var1 + 1
 * - Condition evaluation: Var1 == 2 in transition guard
 *
 * Helper Functions (ARCHITECTURE.md Zero Duplication):
 * - EntryExitHelper: Executes multiple exit handlers as independent blocks
 * - SendHelper: Validates send target, raises error.execution on invalid target
 * - JSEngine: Evaluates ECMAScript expressions and manages datamodel
 * - GuardHelper: Evaluates transition conditions (Var1 == 2)
 *
 * Key Implementation:
 * - EntryExitHelper::executeExitBlocks() processes both exit handlers sequentially
 * - First exit handler block:
 *   - SendHelper::isInvalidTarget("!invalid") returns true
 *   - Raises error.execution event
 *   - Returns early (stops subsequent content in THIS block only)
 * - Second exit handler block:
 *   - JSEngine evaluates "Var1 + 1" → 2
 *   - Assigns 2 to Var1
 *   - Completes successfully
 * - Both blocks execute independently (W3C SCXML 3.8/3.9 compliance)
 *
 * W3C SCXML Features:
 * - 3.8: Multiple onexit handlers as independent blocks (EntryExitHelper)
 * - 3.9: Error handling in executable content (error.execution)
 * - 5.9.2: ECMAScript condition evaluation (GuardHelper)
 * - 5.10: error.execution event on invalid send target (SendHelper)
 * - 6.2: Send element with target validation (SendHelper)
 *
 * Test Flow:
 * 1. Initialize: Enter s0, Var1 = 1 (ECMAScript datamodel initialization)
 * 2. Eventless transition: Exit s0 (execute 2 exit handlers) → Enter s1
 *    - Exit handler block 1: Send to invalid target → error.execution raised, block stops
 *    - Exit handler block 2: Var1 = Var1 + 1 → Var1 becomes 2 (MUST execute independently)
 * 3. Enter s1: Check Var1 == 2
 *    - If Var1 == 2: Transition to pass (second exit handler executed correctly)
 *    - Otherwise: Transition to fail (second exit handler didn't execute)
 * 4. Success: Final state 'pass' reached
 *
 * Failure Scenarios:
 * - Var1 != 2: Second onexit handler didn't execute (independence violated)
 * - error.execution propagated incorrectly (should not stop second handler)
 */
struct Test378 : public SimpleAotTest<Test378, 378> {
    static constexpr const char *DESCRIPTION =
        "Independent onexit handler execution with error.execution (W3C 3.8/3.9 AOT Static Hybrid)";
    using SM = RSM::Generated::test378::test378;
};

// Auto-register
inline static AotTestRegistrar<Test378> registrar_Test378;

}  // namespace RSM::W3C::AotTests
