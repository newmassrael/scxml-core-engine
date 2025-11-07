#pragma once
#include "SimpleAotTest.h"
#include "test560_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 5.10: _event.data structure with params in ECMAScript datamodel
 *
 * Verifies that when an event is received with key-value pairs (params) in the
 * ECMAScript datamodel, the processor creates the correct structure in _event.data
 * allowing access to parameter values via _event.data.paramName syntax.
 *
 * Test flow:
 * 1. State machine initializes to s0
 * 2. Entry action in s0: Evaluate <param name="aParam" expr="1"> → "1"
 * 3. SendHelper raises Event::Foo with EventWithMetadata(data={"aParam":"1"})
 * 4. processTransition: SystemVariableHelper sets _event.data in JSEngine session
 * 5. Guard evaluation: JSEngine evaluates "_event.data.aParam == 1" → true
 * 6. Transition: s0 → pass (final state)
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and expression evaluation
 * - Uses Helper functions: SendHelper (event generation), EventDataHelper (JSON building),
 *   SystemVariableHelper (_event.data setup), GuardHelper (guard evaluation)
 *
 * W3C SCXML Features:
 * - _event.data structure for event parameters (W3C SCXML 5.10)
 * - <param> expression evaluation with JSEngine (W3C SCXML 5.11.2)
 * - JSON construction from params via EventDataHelper::buildJsonFromParams
 * - Guard condition accessing _event.data properties (W3C SCXML 5.9)
 */
struct Test560 : public SimpleAotTest<Test560, 560> {
    static constexpr const char *DESCRIPTION = "_event.data param structure (W3C 5.10 AOT Static Hybrid)";
    using SM = SCE::Generated::test560::test560;
};

// Auto-register
inline static AotTestRegistrar<Test560> registrar_Test560;

}  // namespace SCE::W3C::AotTests
