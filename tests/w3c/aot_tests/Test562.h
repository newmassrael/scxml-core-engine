#pragma once
#include "SimpleAotTest.h"
#include "test562_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 5.9.2: ECMAScript datamodel content space normalization
 *
 * Verifies that when an event contains text content with extra whitespace in the
 * ECMAScript datamodel, the processor creates a space-normalized string in _event.data.
 *
 * Test flow:
 * 1. State machine initializes to s0
 * 2. Entry action in s0: Send event with content "this is  a  \nstring" (extra spaces and newline)
 * 3. SendHelper sends Event::Foo with content in EventWithMetadata.data
 * 4. Runtime space normalization: "this is  a  \nstring" → "this is a string"
 * 5. SystemVariableHelper sets _event.data as normalized string in JSEngine session
 * 6. Guard evaluation: JSEngine evaluates "_event.data == 'this is a string'" → true
 * 7. Transition: s0 → pass (final state)
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and guard condition evaluation
 * - Uses Helper functions: SendHelper (content sending), EventDataHelper (normalization),
 *   SystemVariableHelper (_event.data setup), GuardHelper (guard evaluation)
 *
 * W3C SCXML Features:
 * - ECMAScript content space normalization (W3C SCXML 5.9.2)
 * - _event.data system variable for event content (W3C SCXML 5.10)
 * - Guard condition with ECMAScript string comparison (W3C SCXML 5.9)
 * - Send action with text content (W3C SCXML 6.2)
 */
struct Test562 : public SimpleAotTest<Test562, 562> {
    static constexpr const char *DESCRIPTION = "ECMAScript content space normalization (W3C 5.9.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test562::test562;
};

// Auto-register
inline static AotTestRegistrar<Test562> registrar_Test562;

}  // namespace RSM::W3C::AotTests
