#pragma once
#include "SimpleAotTest.h"
#include "test578_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 5.9/5.10: ECMAScript _event.data JSON object creation
 *
 * Tests that the processor creates an ECMAScript object _event.data when receiving
 * an event with JSON content. Verifies proper parsing of JSON content into _event
 * system variable for ECMAScript datamodel.
 *
 * Test flow:
 * 1. State machine initializes to s0 with ECMAScript datamodel
 * 2. Entry action: Send event "foo" with JSON content: { "productName" : "bar", "size" : 27 }
 * 3. Event "foo" is received, processor parses JSON and populates _event.data object
 * 4. Transition guard evaluates: cond="_event.data.productName == 'bar'"
 * 5. JSEngine accesses _event.data.productName field from parsed JSON object
 * 6. If _event.data.productName == 'bar' → transition to pass (JSON correctly parsed)
 * 7. If condition fails → timeout transition to fail
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and _event.data field access
 * - Uses Helper functions: SendHelper (event with JSON content), EventDataHelper
 *   (_event.data object population), GuardHelper (ECMAScript guard evaluation)
 *
 * W3C SCXML Features:
 * - ECMAScript datamodel (W3C SCXML 5.9)
 * - _event system variable with data field (W3C SCXML 5.10.1)
 * - JSON content parsing into _event.data (W3C SCXML B.2)
 * - ECMAScript property access in guard conditions (W3C SCXML 5.9.2)
 * - <send> with <content> element (W3C SCXML 6.2)
 */
struct Test578 : public SimpleAotTest<Test578, 578> {
    static constexpr const char *DESCRIPTION = "ECMAScript _event.data JSON object (W3C 5.9/5.10 AOT Static Hybrid)";
    using SM = SCE::Generated::test578::test578;
};

// Auto-register
inline static AotTestRegistrar<Test578> registrar_Test578;

}  // namespace SCE::W3C::AotTests
