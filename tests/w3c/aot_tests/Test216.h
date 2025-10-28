#pragma once

#include "SimpleAotTest.h"
#include "test216_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: Invoke srcexpr runtime evaluation
 *
 * Tests that srcexpr attribute is evaluated at runtime, not at parse time.
 * Initial value of Var1 is 'foo' (would fail if used), but is changed to
 * 'file:test216sub1.scxml' at entry, which should be the value used for invoke.
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and expression evaluation
 * - Runtime srcexpr evaluation for dynamic invoke path determination
 * - Uses Helper functions: InvokeHelper, EventMetadataHelper, GuardHelper
 *
 * W3C SCXML Features:
 * - 6.4 (Invoke): srcexpr attribute for runtime source determination
 * - 5.2 (Datamodel): ECMAScript datamodel with <data> and <assign>
 * - 5.9.2 (Assign): Runtime variable assignment before invoke
 */
struct Test216 : public SimpleAotTest<Test216, 216> {
    static constexpr const char *DESCRIPTION = "srcexpr runtime evaluation (W3C 6.4 AOT Static Hybrid)";
    using SM = RSM::Generated::test216::test216;
};

// Auto-register
inline static AotTestRegistrar<Test216> registrar_Test216;

}  // namespace RSM::W3C::AotTests
