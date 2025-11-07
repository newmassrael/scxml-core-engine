#pragma once
#include "SimpleAotTest.h"
#include "test235_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: done.invoke.id event with correct invoke ID
 *
 * Tests that when an invoked child terminates, the done.invoke event
 * contains the correct ID matching the invoke element's id attribute.
 *
 * Test scenario:
 * - Parent state s0 invokes child with id="foo"
 * - Child has immediate final state (terminates instantly)
 * - Parent expects done.invoke.foo event with correct ID
 * - Any other event (including generic done.invoke) is failure
 *
 * W3C SCXML 6.4.5: done.invoke.id event naming (event.name matches invoke id)
 */
struct Test235 : public SimpleAotTest<Test235, 235> {
    static constexpr const char *DESCRIPTION = "done.invoke.id event (W3C 6.4 AOT Pure Static)";
    using SM = SCE::Generated::test235::test235;
};

// Auto-register
inline static AotTestRegistrar<Test235> registrar_Test235;

}  // namespace SCE::W3C::AotTests
