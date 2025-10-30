#pragma once
#include "SimpleAotTest.h"
#include "test307_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 5.2.2: Late Binding Variable Access Error Handling
 *
 * Tests late binding (binding="late") behavior when accessing undeclared variables.
 * In state s0, accesses Var1 which is not yet declared (declared later in s1).
 * Then in s1, accesses a non-existent substructure of Var1.
 *
 * Key behaviors tested:
 * 1. Late binding allows undeclared variable access without error
 * 2. Non-existent variable access returns undefined/null without raising error.execution
 * 3. Both operations should complete without errors (transition on "foo" and "bar", not "error")
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and expression evaluation (Var1, _event)
 * - Uses Helper functions: EventMetadataHelper (_event variable binding)
 *
 * W3C SCXML Features:
 * - W3C SCXML 5.2.2: Late binding variable declaration scoping
 * - W3C SCXML B.2: Error handling for variable access (no error expected)
 * - W3C SCXML 3.12: Log statements (manual verification component)
 *
 * @note Manual test component: Log output verification required by tester
 *       to confirm consistent behavior between both variable access cases
 */
struct Test307 : public SimpleAotTest<Test307, 307> {
    static constexpr const char *DESCRIPTION = "Late binding variable access (W3C 5.2.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test307::test307;

    // Manual test: final state is "final", not "pass"
    static constexpr auto PASS_STATE = SM::State::Final;
};

// Auto-register
inline static AotTestRegistrar<Test307> registrar_Test307;

}  // namespace RSM::W3C::AotTests
