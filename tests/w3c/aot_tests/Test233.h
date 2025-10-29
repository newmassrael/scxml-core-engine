#pragma once
#include "SimpleAotTest.h"
#include "test233_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.5: finalize element execution before done.invoke event processing
 *
 * Tests that finalize markup runs before the done.invoke event is processed.
 * The invoked child process returns 2 in _event.data.aParam, and the finalize
 * element assigns this value to Var1 before the transition is evaluated.
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript expressions (Var1, _event.data.aParam, guard conditions)
 * - Uses Helper functions: SendHelper, EventDataHelper, ActionExecutorImpl
 *
 * W3C SCXML Features:
 * - W3C SCXML 6.5: finalize element for pre-processing done.invoke data
 * - W3C SCXML 6.4: invoke with inline content (static child state machine)
 * - W3C SCXML 5.9.2: assign element for datamodel updates
 * - W3C SCXML 3.12.1: cond attribute for conditional transitions
 * - W3C SCXML 5.10.1: send with param element for event data
 *
 * Test Flow:
 * 1. Parent initializes Var1=1
 * 2. Parent invokes child state machine with finalize element
 * 3. Child immediately sends childToParent event with param aParam=2
 * 4. Parent's finalize executes: Var1 = _event.data.aParam (Var1 becomes 2)
 * 5. Parent evaluates transition: cond="Var1 == 2" → true → pass state
 */
struct Test233 : public SimpleAotTest<Test233, 233> {
    static constexpr const char *DESCRIPTION = "finalize before done.invoke (W3C 6.5 AOT Static Hybrid)";
    using SM = RSM::Generated::test233::test233;
};

// Auto-register
inline static AotTestRegistrar<Test233> registrar_Test233;

}  // namespace RSM::W3C::AotTests
