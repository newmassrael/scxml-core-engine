#pragma once
#include "SimpleAotTest.h"
#include "test551_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 5.2.2: Early binding with inline content
 *
 * Verifies that inline content can be used to assign a value to a variable
 * in early binding mode. The <data> element in state s1 contains an array
 * literal [1,2,3] which is evaluated via JSEngine before state machine
 * execution starts (early binding).
 *
 * Test flow:
 * 1. State machine starts with binding="early"
 * 2. Data variable Var1 is initialized from inline content [1,2,3]
 * 3. Initial state s0 checks if Var1 is defined using 'typeof Var1 !== 'undefined''
 * 4. If Var1 is bound, transitions to pass; otherwise to fail
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and expression evaluation
 * - Uses Helper functions: GuardHelper (for condition evaluation)
 *
 * W3C SCXML Features:
 * - Early binding variable initialization (5.2.2)
 * - Inline content in <data> element (B.2)
 * - ECMAScript typeof operator (B.2)
 * - Guard condition evaluation (3.13)
 */
struct Test551 : public SimpleAotTest<Test551, 551> {
    static constexpr const char *DESCRIPTION = "Early binding inline content (W3C 5.2.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test551::test551;
};

// Auto-register
inline static AotTestRegistrar<Test551> registrar_Test551;

}  // namespace RSM::W3C::AotTests
