#pragma once
#include "SimpleAotTest.h"
#include "test570_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 3.12.1: Parallel state completion with done.state.id events
 *
 * Verifies that when all children of a parallel state reach final states, the processor
 * generates a done.state.id event where id is the id of the parallel state. Tests proper
 * parallel state completion semantics with ECMAScript datamodel state tracking.
 *
 * Test flow:
 * 1. State machine initializes to parallel state p0 (with children p0s1, p0s2)
 * 2. Entry action: Initialize Var1 = 0 in ECMAScript datamodel
 * 3. p0s1 transitions to p0s1final (first child reaches final state)
 * 4. p0s2 transitions through p0s21, executes assign Var1 = 1
 * 5. p0s2 transitions to p0s2final (second child reaches final state)
 * 6. When all children final: Processor generates done.state.p0 event (W3C SCXML 3.12.1)
 * 7. Transition from p0: Guard evaluates cond="Var1 == 1" with JSEngine
 * 8. If Var1 == 1 → transition to pass (successful state tracking)
 * 9. If Var1 != 1 → transition to fail
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel (data initialization, assignment, guard evaluation)
 * - Uses Helper functions: DataModelInitHelper (Var1 initialization), AssignHelper (Var1 assignment),
 *   GuardHelper (guard evaluation), DoneDataHelper (done.state.id event generation)
 *
 * W3C SCXML Features:
 * - Parallel state completion semantics (W3C SCXML 3.4)
 * - done.state.id event generation (W3C SCXML 3.12.1)
 * - ECMAScript datamodel with variable tracking (W3C SCXML B.2)
 * - Guard conditions with ECMAScript expressions (W3C SCXML 5.9)
 * - Assignment actions (W3C SCXML 5.4)
 */
struct Test570 : public SimpleAotTest<Test570, 570> {
    static constexpr const char *DESCRIPTION =
        "Parallel state completion with done.state.id (W3C 3.12.1 AOT Static Hybrid)";
    using SM = SCE::Generated::test570::test570;
};

// Auto-register
inline static AotTestRegistrar<Test570> registrar_Test570;

}  // namespace SCE::W3C::AotTests
