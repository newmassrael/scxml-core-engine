#pragma once
#include "SimpleAotTest.h"
#include "test533_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 3.13: Internal transition exit set for non-compound states
 *
 * Tests that if a transition has 'type' of "internal", but its source state is
 * not a compound state, its exit set is defined as if it had 'type' of "external".
 *
 * W3C SCXML 3.13 specifies that internal transitions are only truly internal when:
 * 1. The source state is a compound state (has child states)
 * 2. All target states are proper descendants of the source state
 *
 * If the source state is NOT compound (atomic state with no children), an internal
 * transition behaves as an external transition, meaning it exits and re-enters the
 * source state even though type="internal".
 *
 * Test structure:
 * - State s0: Non-compound (atomic) state with 9 substates (s01-s09)
 * - Internal transition from s0 to s01 with type="internal"
 * - Because s0 is NOT compound (s01-s09 are siblings, not children), the transition
 *   exits s0 and re-enters it, executing onexit and onentry handlers
 * - Counters (Var1-Var4) track exit/entry execution to validate behavior
 *
 * Expected behavior:
 * 1. Enter s0 → s01 (initial state), Var1-Var4 initialized
 * 2. Event "foo" triggers internal transition from s0 to s01
 * 3. Because s0 is NOT compound, transition behaves as external:
 *    - Exit s01 (increment Var2)
 *    - Exit s0 (increment Var1) - KEY VERIFICATION POINT
 *    - Execute transition (increment Var3)
 *    - Enter s0 (increment Var4) - KEY VERIFICATION POINT
 *    - Enter s01
 * 4. Event "bar" validates counters and transitions to pass/fail
 *
 * This test ensures that the "internal" type attribute is correctly interpreted
 * based on the state topology, not just the attribute value.
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * ✅ Static Hybrid Strategy:
 * - State machine structure: Fully static (compile-time known states/transitions)
 * - JSEngine for ECMAScript datamodel: Variable initialization and arithmetic operations
 * - Uses Helper functions: TransitionHelper for exit set calculation
 * - Zero Duplication: TransitionHelper.computeExitSet() shared with Interpreter
 *
 * W3C SCXML Features:
 * - W3C SCXML 3.13: Internal transition semantics for non-compound states
 * - W3C SCXML B.2: ECMAScript datamodel for counter variables
 * - W3C SCXML 5.3: Assign action for variable updates
 * - W3C SCXML E: Conditional expressions for validation
 *
 * Key Implementation Detail:
 * The transition exit set calculation must check if the source state is compound.
 * For non-compound states, even type="internal" transitions must exit and re-enter
 * the source state, identical to external transition behavior.
 */
struct Test533 : public SimpleAotTest<Test533, 533> {
    static constexpr const char *DESCRIPTION =
        "Internal transition exit set for non-compound states (W3C 3.13 AOT Static Hybrid)";
    using SM = SCE::Generated::test533::test533;
};

// Auto-register
inline static AotTestRegistrar<Test533> registrar_Test533;

}  // namespace SCE::W3C::AotTests
