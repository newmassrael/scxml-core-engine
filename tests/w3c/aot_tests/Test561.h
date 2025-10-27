#pragma once
#include "SimpleAotTest.h"
#include "test561_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 5.9.2: ECMAScript DOM object creation for XML event data
 *
 * Verifies that when an event contains XML content in the ECMAScript datamodel,
 * the processor creates an ECMAScript DOM object in _event.data that supports
 * standard DOM manipulation methods (getElementsByTagName, getAttribute).
 *
 * Test flow:
 * 1. State machine initializes to s0
 * 2. Entry action in s0: Send event with XML content (books/book elements)
 * 3. SendHelper sends Event::Foo with XML content in EventWithMetadata.data
 * 4. JSEngine receives event and creates DOM object from XML content
 * 5. SystemVariableHelper sets _event.data as DOM object in JSEngine session
 * 6. Guard evaluation: JSEngine evaluates "_event.data.getElementsByTagName('book')[1].getAttribute('title') ==
 * 'title2'"
 * 7. DOM methods execute: getElementsByTagName returns node list, getAttribute accesses attribute
 * 8. Transition: s0 → pass (final state)
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and XML DOM manipulation
 * - Uses Helper functions: SendHelper (XML content sending), EventDataHelper (XML parsing),
 *   SystemVariableHelper (_event.data DOM setup), GuardHelper (DOM expression evaluation)
 *
 * W3C SCXML Features:
 * - ECMAScript DOM object for _event.data (W3C SCXML 5.9.2, Appendix B)
 * - XML content in send action (W3C SCXML 6.2)
 * - DOM manipulation methods: getElementsByTagName, getAttribute
 * - Guard condition with complex ECMAScript expressions (W3C SCXML 5.9)
 *
 * Infrastructure:
 * - Code generator serializes <content> child XML elements (scxml_parser.py:696-703)
 * - Matches <data> element XML parsing behavior for consistency
 * - Enables full W3C SCXML B.2 ECMAScript datamodel compliance
 */
struct Test561 : public SimpleAotTest<Test561, 561> {
    static constexpr const char *DESCRIPTION = "ECMAScript XML DOM event data (W3C 5.9.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test561::test561;
};

// Auto-register
inline static AotTestRegistrar<Test561> registrar_Test561;

}  // namespace RSM::W3C::AotTests
