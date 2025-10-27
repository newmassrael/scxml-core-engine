#pragma once
#include "SimpleAotTest.h"
#include "test579_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 3.8: History default content execution order
 *
 * Tests that default history content is executed only when no stored history exists.
 * Verifies the interaction between initial transitions, history pseudo-states, and
 * ECMAScript datamodel variables to ensure proper history default transition behavior.
 *
 * Test flow:
 * 1. State machine initializes to s0 with ECMAScript datamodel (Var1 = 0)
 * 2. Schedules delayed timeout event (1s) via <send delay="1s" event="timeout"/> (W3C SCXML 6.2)
 * 3. First visit to history state: No stored history exists
 * 4. History default transition executes: Var1 incremented to 1 (W3C SCXML 3.8)
 * 5. Exit and re-enter parent state: History is now stored
 * 6. Second visit to history state: Stored history exists
 * 7. History default transition does NOT execute: Var1 remains 1 (W3C SCXML 3.8)
 * 8. Waits for scheduled timeout event via tick() polling loop (W3C SCXML 6.2)
 * 9. Guard condition evaluates: cond="Var1 == 1" with JSEngine
 * 10. If Var1 == 1 → transition to pass (history default correctly skipped)
 * 11. If Var1 != 1 → transition to fail (history default incorrectly re-executed)
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions/history)
 * - JSEngine for ECMAScript datamodel (variable initialization, assignment, guard evaluation)
 * - Uses Helper functions: DataModelInitHelper (Var1 initialization), AssignHelper (Var1 += 1),
 *   GuardHelper (guard evaluation), HistoryHelper (history state management)
 *
 * W3C SCXML Features:
 * - Delayed send with event scheduler (W3C SCXML 6.2)
 * - History state default transitions (W3C SCXML 3.8)
 * - History pseudo-state behavior (W3C SCXML 3.11)
 * - ECMAScript datamodel with variable tracking (W3C SCXML B.2)
 * - Guard conditions with ECMAScript expressions (W3C SCXML 5.9)
 * - Assignment actions (W3C SCXML 5.3)
 */
struct Test579 : public ScheduledAotTest<Test579, 579> {
    static constexpr const char *DESCRIPTION =
        "History default content execution with delayed send (W3C 3.8 + 6.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test579::test579;
};

// Auto-register
inline static AotTestRegistrar<Test579> registrar_Test579;

}  // namespace RSM::W3C::AotTests
