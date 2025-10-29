#pragma once
#include "ScheduledAotTest.h"
#include "test187_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4.5: Delayed send cancellation on session termination
 *
 * Tests that delayed <send> events are cancelled when the sending session terminates.
 * A child SCXML session is invoked that sends a delayed event to the parent, then
 * immediately exits before the delay expires. The parent should NOT receive the event.
 *
 * Test flow:
 * 1. Parent state s0 invokes child SCXML session with inline content
 * 2. Parent schedules delayed timeout event (1s) via <send event="timeout" delay="1s"/> (W3C SCXML 6.2)
 * 3. Child state sub0 schedules delayed event to parent: <send event="childToParent" target="#_parent" delay=".5"/>
 * 4. Child immediately transitions to subFinal (terminates before 0.5s delay)
 * 5. W3C SCXML 6.4.5: Child termination MUST cancel pending delayed send
 * 6. Parent waits for 1s timeout
 * 7. If childToParent received → fail (delayed send not cancelled)
 * 8. If timeout received → pass (delayed send correctly cancelled)
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 *
 * - Fully static state machine (compile-time states/transitions)
 * - No JSEngine needed (all event names, delays, targets are static literals)
 * - Uses Helper functions: SendHelper (delayed send, target resolution),
 *   InvokeHelper (child SCXML session management), EventSchedulerHelper (delay processing)
 * - Zero runtime expression evaluation overhead
 *
 * W3C SCXML Features:
 * - Invoke with inline <content> child SCXML (W3C SCXML 6.4)
 * - Delayed send with event scheduler (W3C SCXML 6.2)
 * - Target resolution with #_parent (W3C SCXML 6.2.4)
 * - Send cancellation on session termination (W3C SCXML 6.4.5)
 * - Event queue processing order (W3C SCXML 3.13)
 */
struct Test187 : public ScheduledAotTest<Test187, 187> {
    static constexpr const char *DESCRIPTION =
        "Delayed send cancellation on session termination (W3C 6.4.5 AOT Pure Static)";
    using SM = RSM::Generated::test187::test187;
};

// Auto-register
inline static AotTestRegistrar<Test187> registrar_Test187;

}  // namespace RSM::W3C::AotTests
