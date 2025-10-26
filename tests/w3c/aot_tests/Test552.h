#pragma once
#include "SimpleAotTest.h"
#include "test552_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 5.2.2: External data loading via src attribute
 *
 * Verifies that <data src="file:..."> can load content from external files
 * and assign to datamodel variables. The <data> element uses src attribute
 * to load from test552.txt, which is then validated using ECMAScript typeof
 * operator in a transition guard.
 *
 * Test flow:
 * 1. State machine starts with binding="early"
 * 2. Data variable Var1 is loaded from external file test552.txt (value: "2")
 * 3. Initial state s0 checks if Var1 is defined using 'typeof Var1 !== 'undefined''
 * 4. If Var1 is successfully loaded, transitions to pass; otherwise to fail
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and expression evaluation
 * - Uses Helper functions: GuardHelper (for condition evaluation), FileLoadingHelper (for src loading)
 *
 * W3C SCXML Features:
 * - External data loading via src attribute (5.2.2)
 * - Early binding with external source (5.2.2)
 * - ECMAScript typeof operator (B.2)
 * - Guard condition evaluation (3.13)
 */
struct Test552 : public SimpleAotTest<Test552, 552> {
    static constexpr const char *DESCRIPTION = "External data src attribute (W3C 5.2.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test552::test552;
};

// Auto-register
inline static AotTestRegistrar<Test552> registrar_Test552;

}  // namespace RSM::W3C::AotTests
