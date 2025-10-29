#pragma once
#include "ScheduledAotTest.h"
#include "test237_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: invoke cancellation on state exit
 *
 * Tests that when a parent state exits while an invoked child is running,
 * the invocation is cancelled and no done.invoke event is received.
 *
 * Test scenario:
 * - Parent state s0 invokes child with 2-second delay to termination
 * - Parent transitions to s1 after 1 second (exits s0, cancelling invoke)
 * - s1 waits 1.5 seconds for any events
 * - If done.invoke received → fail (cancellation didn't work)
 * - If timeout2 fires without done.invoke → pass (cancellation worked)
 *
 * W3C SCXML 6.4: Invoke mechanism with automatic cancellation on state exit
 * W3C SCXML 6.2: Delayed send requires event scheduler polling (ScheduledAotTest)
 */
struct Test237 : public ScheduledAotTest<Test237, 237> {
    static constexpr const char *DESCRIPTION = "invoke cancellation (W3C 6.4 AOT Pure Static)";
    using SM = RSM::Generated::test237::test237;
};

// Auto-register
inline static AotTestRegistrar<Test237> registrar_Test237;

}  // namespace RSM::W3C::AotTests
