#pragma once
#include "SimpleAotTest.h"
#include "test234_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4.6: Finalize block only executes in invoking state
 *
 * Tests that when multiple parallel states have invocations with finalize blocks,
 * only the finalize block in the state receiving the done.invoke event executes.
 *
 * Test scenario:
 * - Parallel state p0 with two child states (p01, p02)
 * - p01 invokes child that returns event with aParam=2
 * - p02 invokes child that sleeps (no event)
 * - Only p01's finalize should execute (Var1 = 2)
 * - p02's finalize should NOT execute (Var2 remains 1)
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel (_event.data.aParam, conditions)
 * - Uses Helper functions:
 *   - InvokeHelper: Shared invoke lifecycle management
 *   - SendSchedulingHelper: Delayed send scheduling (timeout event)
 *   - GuardHelper: Condition evaluation (Var1 == 2, Var2 == 1)
 *   - DatamodelHelper: Variable initialization and assignment
 *
 * W3C SCXML Features:
 * - 6.4.6: Finalize execution scoped to invoking state (not all parallel states)
 * - 5.8: Parallel state execution with independent invocations
 * - 6.2: Delayed send for timeout
 * - 5.10: Event data parameter access (_event.data.aParam)
 */
struct Test234 : public SimpleAotTest<Test234, 234> {
    static constexpr const char *DESCRIPTION = "Finalize only in invoking state (W3C 6.4.6 AOT Static Hybrid)";
    using SM = RSM::Generated::test234::test234;
};

// Auto-register
inline static AotTestRegistrar<Test234> registrar_Test234;

}  // namespace RSM::W3C::AotTests
