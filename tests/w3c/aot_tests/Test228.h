#pragma once
#include "ScheduledAotTest.h"
#include "test228_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.3.1: _event.invokeid contains invoke ID in done.invoke event
 *
 * Tests that when an invoked child completes, the done.invoke event's
 * _event.invokeid field contains the correct invoke ID for parent access.
 *
 * Test scenario:
 * - Parent invokes child with id="foo"
 * - Child completes immediately (final state)
 * - Parent receives done.invoke event
 * - Parent transition assigns Var1 = _event.invokeid
 * - Guard checks Var1 == 'foo' to verify invokeId propagation
 *
 * W3C SCXML 6.3.1: "The 'invokeid' field of the event is set to the invoke id of the invocation that was finished"
 */
struct Test228 : public ScheduledAotTest<Test228, 228> {
    static constexpr const char *DESCRIPTION = "Invoke ID in done.invoke event (W3C 6.3.1 AOT Static Hybrid)";
    using SM = RSM::Generated::test228::test228;
};

// Auto-register
inline static AotTestRegistrar<Test228> registrar_Test228;

}  // namespace RSM::W3C::AotTests
