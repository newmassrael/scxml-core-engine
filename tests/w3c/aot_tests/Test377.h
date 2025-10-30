#pragma once
#include "SimpleAotTest.h"
#include "test377_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 3.8: Multiple onexit handlers execution in document order
 *
 * Test Description:
 * - Verifies that multiple onexit handlers are executed in document order
 * - State s0 has two onexit handlers: first raises event1, second raises event2
 * - State s1 expects event1 first (transitions to s2)
 * - State s2 expects event2 next (transitions to pass)
 * - Any other event order leads to fail state
 *
 * W3C SCXML 3.8 Specification:
 * - Exit handlers execute when a state is exited during a transition
 * - Multiple exit handlers in a single state execute in document order
 * - All exit handlers complete before entering new states
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 *
 * Static State Machine Structure:
 * - Compile-time states: s0, s1, s2, pass, fail (State enum)
 * - Compile-time events: event1, event2 (Event enum)
 * - Compile-time transitions: s0→s1, s1→s2→pass, any mismatch→fail
 * - No JSEngine needed (all values are static literals)
 *
 * Helper Functions (ARCHITECTURE.md Zero Duplication):
 * - EntryExitHelper: Executes multiple exit handlers in document order
 * - InternalEventHelper: Processes raise events (event1, event2) in queue
 * - StateTransitionHelper: Handles hierarchical transitions (s0→s1→s2)
 *
 * Key Implementation:
 * - EntryExitHelper::executeExitActions() processes both exit handlers sequentially
 * - First exit handler: <raise event="event1"/> → queued to internal queue
 * - Second exit handler: <raise event="event2"/> → queued after event1
 * - State machine processes event1 first (s1 transition), then event2 (s2 transition)
 * - Document order preservation ensures correct event sequence
 *
 * W3C SCXML Features:
 * - 3.8: Multiple onexit handlers in document order (EntryExitHelper)
 * - 5.1: Raise element for internal event generation (InternalEventHelper)
 * - 3.12.1: Event queue processing (event1 before event2)
 * - 3.13: Transition selection based on document order
 *
 * Test Flow:
 * 1. Initialize: Enter s0
 * 2. Eventless transition: Exit s0 (execute 2 exit handlers) → Enter s1
 *    - Exit handler 1: raise event1 → internal queue
 *    - Exit handler 2: raise event2 → internal queue
 * 3. Process event1: Transition s1→s2 (event1 expected)
 * 4. Process event2: Transition s2→pass (event2 expected)
 * 5. Success: Final state 'pass' reached
 *
 * Failure Scenarios (leading to 'fail' state):
 * - event2 arrives before event1 (exit handler order violated)
 * - Any unexpected event arrives (wildcard * transitions to fail)
 */
struct Test377 : public SimpleAotTest<Test377, 377> {
    static constexpr const char *DESCRIPTION = "Multiple onexit handlers in document order (W3C 3.8 AOT Pure Static)";
    using SM = RSM::Generated::test377::test377;
};

// Auto-register
inline static AotTestRegistrar<Test377> registrar_Test377;

}  // namespace RSM::W3C::AotTests
