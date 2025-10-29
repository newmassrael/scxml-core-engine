#pragma once
#include "ScheduledAotTest.h"
#include "test580_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 3.11: History state never in configuration (parallel states)
 *
 * Tests that history pseudo-states never appear as part of the active configuration.
 * Uses In() predicate within parallel state to verify history state is not active.
 *
 * Test flow:
 * 1. Parallel state p1 initializes with two regions: s0 and s1
 * 2. Schedules delayed timeout event (2s) via <send delay="2s" event="timeout"/> (W3C SCXML 6.2)
 * 3. Region s0: Checks In('sh1') - should NEVER be true
 * 4. Region s1: Contains history state sh1 with initial transition to s11
 * 5. First visit: sh1 initial transition → s11 → s12 (history stored)
 * 6. s11 and s1 both check In('sh1') - should be false
 * 7. Var1 == 0 → transition back to sh1 (history restoration)
 * 8. Second visit: sh1 restores to s12 (no initial transition)
 * 9. s1 exit increments Var1 to 1
 * 10. Var1 == 1 → transition to pass
 * 11. If In('sh1') ever true → fail (history state in configuration)
 * 12. If timeout occurs → fail (took too long)
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static parallel state structure (compile-time parallel regions)
 * - JSEngine for ECMAScript datamodel (Var1 tracking, In() predicate)
 * - Uses Helper functions: ParallelStateHelper (parallel region coordination),
 *   HistoryHelper (history state management), InPredicateHelper (In() evaluation),
 *   GuardHelper (guard evaluation), AssignHelper (Var1 += 1)
 *
 * W3C SCXML Features:
 * - Parallel states with multiple regions (W3C SCXML 3.4)
 * - History pseudo-states (W3C SCXML 3.11)
 * - In() predicate for state checking (W3C SCXML B.2)
 * - Delayed send with event scheduler (W3C SCXML 6.2)
 * - ECMAScript datamodel (W3C SCXML B.2)
 * - Guard conditions with expressions (W3C SCXML 5.9)
 */
struct Test580 : public ScheduledAotTest<Test580, 580> {
    static constexpr const char *DESCRIPTION =
        "History state never in configuration with parallel states (W3C 3.11 AOT Static Hybrid)";
    using SM = RSM::Generated::test580::test580;
};

// Auto-register
inline static AotTestRegistrar<Test580> registrar_Test580;

}  // namespace RSM::W3C::AotTests
