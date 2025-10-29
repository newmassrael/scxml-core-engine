#pragma once

#include "ScheduledAotTest.h"
#include "test216_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: Invoke srcexpr runtime evaluation
 *
 * Tests that srcexpr attribute is evaluated at runtime, not at parse time.
 * Initial value of Var1 is 'foo' (would fail if used), but is changed to
 * 'file:test216sub1.scxml' at entry, which should be the value used for invoke.
 *
 * Uses ScheduledAotTest for runUntilCompletion() to process:
 * - Deferred hybrid invoke execution (W3C SCXML 6.4)
 * - Interpreter child state machine lifecycle
 * - Event scheduler polling for timeout
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and srcexpr evaluation
 * - Runtime srcexpr evaluation for dynamic invoke path determination
 * - Hybrid invoke: AOT parent + Interpreter child (ARCHITECTURE.md Hybrid Strategy)
 * - Uses Helper functions: InvokeHelper, FileLoadingHelper
 *
 * W3C SCXML Features:
 * - 6.4 (Invoke): srcexpr attribute for runtime source determination
 * - 3.12.1 (Invoke ID): Automatic ID generation in "stateid.platformid.index" format
 *   (index suffix ensures uniqueness for multiple invokes in same state)
 * - 5.2 (Datamodel): ECMAScript datamodel with <data> and <assign>
 * - 5.9.2 (Assign): Runtime variable assignment before invoke
 */
struct Test216 : public ScheduledAotTest<Test216, 216> {
    static constexpr const char *DESCRIPTION = "srcexpr runtime evaluation (W3C 6.4 AOT Static Hybrid)";
    using SM = RSM::Generated::test216::test216;
};

// Auto-register
inline static AotTestRegistrar<Test216> registrar_Test216;

}  // namespace RSM::W3C::AotTests
