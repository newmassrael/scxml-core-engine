#pragma once
#include "ScheduledAotTest.h"
#include "test252_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: Invoke cancellation - events from cancelled child are ignored
 *
 * Tests that events received from an invoked process are not processed once the invoke is cancelled.
 * The child process attempts to send "childToParent" event in an onexit handler.
 * If the parent receives and processes it, the test fails. Timeout indicates success.
 *
 * Test Scenario:
 * 1. Parent enters s0/s01, sends timeout (1s delay), starts invoke of child SCXML
 * 2. Parent sends "foo" event immediately, triggering transition to s02
 * 3. Transition to s02 exits s01, which cancels the invoke (W3C SCXML 6.4)
 * 4. Child process: Sets timeout (0.5s delay), transitions to subFinal
 * 5. Child onexit handler: Sends "childToParent" event to #_parent
 * 6. Parent MUST NOT process "childToParent" (invoke was cancelled)
 * 7. If parent receives "childToParent" or "done.invoke" → fail
 * 8. If 1s timeout occurs without receiving cancelled child events → pass
 *
 * W3C SCXML 6.4 Requirements:
 * - Invoke is cancelled when the invoking state is exited
 * - Events from cancelled invocations MUST be discarded by the parent
 * - Onexit handlers in cancelled child execute, but parent ignores their events
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 *
 * Static elements (compile-time):
 * - State machine structure (State enum: S0, S01, S02, Pass, Fail)
 * - Event types (Event enum: Timeout, Foo, ChildToParent, Done_invoke)
 * - Transition logic (switch-case)
 * - Inline child SCXML content (compile-time constant)
 *
 * Dynamic elements (runtime):
 * - Event scheduler for delayed send (1s, 0.5s timeouts)
 * - Invoke lifecycle management (InvokeHelper: defer/cancel/execute)
 * - Child→parent event filtering (cancelled invoke events discarded)
 * - Child state machine execution (separate StaticExecutionEngine instance)
 *
 * W3C SCXML Features:
 * - 6.4: Invoke with inline content
 * - 6.4: Invoke cancellation on state exit
 * - 6.2.6: #_parent target for child→parent event communication
 * - 6.2: Delayed send with event scheduler
 * - 3.8: Onexit handlers (execute even in cancelled child)
 *
 * Helper Functions (Zero Duplication with Interpreter):
 * - InvokeHelper: Defer/cancel/execute invoke lifecycle (W3C SCXML 6.4)
 * - SendSchedulingHelper: Schedule delayed send events (W3C SCXML 6.2)
 * - SendHelper: Process #_parent target and event routing
 * - EventDataHelper: Event metadata and _event variable binding
 * - HierarchicalStateHelper: LCA calculation for hierarchical transitions
 *
 * Expected Result: Pass (timeout occurs without receiving cancelled child events)
 */
struct Test252 : public ScheduledAotTest<Test252, 252> {
    static constexpr const char *DESCRIPTION = "Invoke cancellation ignores child events (W3C 6.4 AOT Pure Static)";
    using SM = RSM::Generated::test252::test252;
};

// Auto-register
inline static AotTestRegistrar<Test252> registrar_Test252;

}  // namespace RSM::W3C::AotTests
