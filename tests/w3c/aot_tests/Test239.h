#pragma once
#include "SimpleAotTest.h"
#include "test239_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: invoke with src attribute and inline content
 *
 * Tests that SCXML invocation works with both external file loading (src attribute)
 * and inline content definition (<content><scxml>...</scxml></content>).
 *
 * Test scenario:
 * - State s01 invokes external file via src="file:test239sub1.scxml"
 * - On done.invoke, transition to s02
 * - State s02 invokes inline content child (identical to test239sub1)
 * - On done.invoke, transition to pass
 * - Timeout after 2 seconds → fail
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 * - Fully static state machine (compile-time states/transitions)
 * - No JSEngine needed (no data variables or expressions)
 * - Uses Helper functions: InvokeHelper (src file loading + inline content)
 *
 * W3C SCXML Features:
 * - W3C SCXML 6.4: Invoke with src attribute (external file loading)
 * - W3C SCXML 6.4: Invoke with inline content (<content> element)
 * - W3C SCXML 6.3.1: done.invoke event on child completion
 * - W3C SCXML 6.2: Delayed send with timeout
 */
struct Test239 : public SimpleAotTest<Test239, 239> {
    static constexpr const char *DESCRIPTION = "invoke src + content (W3C 6.4 AOT Pure Static)";
    using SM = RSM::Generated::test239::test239;
};

// Auto-register
inline static AotTestRegistrar<Test239> registrar_Test239;

}  // namespace RSM::W3C::AotTests
