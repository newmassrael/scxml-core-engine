#pragma once
#include "SimpleAotTest.h"
#include "test506_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 5.9.2: Internal transition with non-descendant target behaves as external
 *
 * Validates that an internal transition (type="internal") whose target is NOT
 * a proper descendant of its source state behaves like an external transition,
 * meaning it exits and re-enters the source state.
 *
 * This test verifies the W3C SCXML 5.9.2 specification that internal transitions
 * only avoid exiting the source state when targeting proper descendants. When
 * the target is the source state itself or any other non-descendant, the
 * internal transition must behave as an external transition.
 *
 * Expected behavior:
 * - State s1 transitions to s2, entering child s21 (Var1=1, Var2=1 from initial entry)
 * - Event "foo" triggers internal transition from s2 to s2 (type="internal" target="s2")
 * - Since s2 is NOT a proper descendant of itself, this behaves as external transition
 * - External behavior: exit s21, exit s2, execute transition actions (Var3=1), re-enter s2, re-enter s21
 * - Result: Var1=2 (s2 exited twice), Var2=2 (s21 exited twice), Var3=1 (transition taken once)
 * - Event "bar" validates counters and transitions to pass state
 *
 * Uses Static Hybrid approach: static state machine structure with
 * runtime ECMAScript expression evaluation via JSEngine for counter tracking.
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication Principle: Uses ParallelTransitionHelper and InternalTransitionHelper
 *   for shared internal transition logic between Interpreter and AOT engines
 * - Single Source of Truth: Internal transition semantics centralized in InternalTransitionHelper
 * - W3C SCXML Perfect Compliance: Fully implements W3C SCXML 5.9.2 specification
 */
struct Test506 : public SimpleAotTest<Test506, 506> {
    static constexpr const char *DESCRIPTION =
        "Internal transition non-descendant target behaves as external (W3C 5.9.2 AOT Static Hybrid)";
    using SM = SCE::Generated::test506::test506;
};

// Auto-register
inline static AotTestRegistrar<Test506> registrar_Test506;

}  // namespace SCE::W3C::AotTests
