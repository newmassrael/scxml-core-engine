#pragma once
#include "SimpleAotTest.h"
#include "test294_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 5.7.2: Donedata with param and content
 *
 * Tests that:
 * - <param> inside <donedata> ends up in the data field of the done event (_event.data.Var1)
 * - <content> inside <donedata> sets the full value of the event.data field (_event.data)
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and expression evaluation
 *   - Condition evaluation: `_event.data.Var1 == 1`, `_event.data == 'foo'`
 *   - Donedata param expr: `expr="1"`
 *   - Donedata content expr: `'foo'`
 * - Uses Helper functions:
 *   - DoneDataHelper: Processes donedata param and content (shared with Interpreter)
 *   - EventMetadataHelper: Binds _event.data field (shared with Interpreter)
 *   - GuardHelper: Evaluates transition conditions (shared with Interpreter)
 *
 * W3C SCXML Features:
 * - 5.7.2: Final state donedata with <param> element (name/expr attributes)
 * - 5.7.2: Final state donedata with <content> element (full data replacement)
 * - 5.9: Done events for compound states (done.state.s0, done.state.s1)
 * - B.2.1: _event.data field access in ECMAScript datamodel
 */
struct Test294 : public SimpleAotTest<Test294, 294> {
    static constexpr const char *DESCRIPTION = "donedata with param and content (W3C 5.7.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test294::test294;
};

// Auto-register
inline static AotTestRegistrar<Test294> registrar_Test294;

}  // namespace RSM::W3C::AotTests
