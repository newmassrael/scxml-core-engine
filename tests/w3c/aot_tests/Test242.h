#pragma once
#include "ScheduledAotTest.h"
#include "test242_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: invoke src + inline content consistency validation
 *
 * Tests that markup specified by 'src' and by '<content>' is treated identically.
 * Either we get done.invoke in both cases or in neither case (timeout).
 *
 * Test scenario:
 * - State s0 invokes child with src="file:test242sub1.scxml"
 *   - Child is a simple final state
 *   - Timeout after 1s if no done.invoke
 * - On done.invoke event, transition to s02 (src worked)
 * - On timeout1 event, transition to s03 (src failed)
 * - State s02 invokes child with inline <content> (identical to test242sub1.scxml)
 *   - Must receive done.invoke (since s0 received done.invoke)
 *   - On done.invoke → pass (consistent behavior)
 *   - On timeout2 → fail (inconsistent)
 * - State s03 invokes child with inline <content> (identical to test242sub1.scxml)
 *   - Must receive timeout (since s0 received timeout1)
 *   - On timeout3 → pass (consistent behavior)
 *   - On done.invoke → fail (inconsistent)
 *
 * Expected behavior: Either both invokes succeed (s0→s02→pass) or both fail (s0→s03→pass).
 * The test validates src and inline content have identical semantics.
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 * - Fully static state machine (compile-time states/transitions)
 * - No JSEngine needed (all values are static literals)
 * - Uses Helper functions:
 *   - InvokeHelper: defer/cancel/execute pattern for invoke lifecycle (W3C SCXML 6.4)
 *   - SendSchedulingHelper: delayed send scheduling (W3C SCXML 6.2)
 *   - EventMetadataHelper: done.invoke event generation (W3C SCXML 6.3.1)
 *   - SendHelper: send to parent communication
 *
 * W3C SCXML Features:
 * - W3C SCXML 6.4: Invoke with src attribute (external SCXML file)
 * - W3C SCXML 6.4: Invoke with inline content (<content><scxml>...</scxml></content>)
 * - W3C SCXML 6.3.1: done.invoke event generation
 * - W3C SCXML 6.2: Delayed send with timeout
 * - W3C SCXML 3.7.3: Final state in child state machine
 *
 * Note: This test requires test242sub1.scxml (child state machine file).
 * The child is a simple state machine with only a final state.
 * Both src and inline content reference identical child state machines to verify consistency.
 */
struct Test242 : public ScheduledAotTest<Test242, 242> {
    static constexpr const char *DESCRIPTION = "invoke src + content consistency (W3C 6.4 AOT Pure Static)";
    using SM = SCE::Generated::test242::test242;
};

// Auto-register
inline static AotTestRegistrar<Test242> registrar_Test242;

}  // namespace SCE::W3C::AotTests
