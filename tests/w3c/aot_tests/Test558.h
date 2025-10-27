#pragma once
#include "SimpleAotTest.h"
#include "test558_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML B.2: ECMAScript whitespace normalization for non-XML content
 *
 * Verifies that in the ECMAScript datamodel, when <data> child content is not XML,
 * or when XML is loaded via src=, the processor treats the value as a string and
 * performs whitespace normalization before assignment.
 *
 * Test flow:
 * 1. State machine initializes with two variables:
 *    - var1: Inline content "this  is \na string" (multiple spaces and newline)
 *    - var2: External file test558.txt with same whitespace-containing content
 * 2. State s0 checks if var1 == 'this is a string' (normalized)
 * 3. State s1 checks if var2 == 'this is a string' (normalized)
 * 4. Both conditions must pass to reach final state
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and whitespace normalization
 * - Uses Helper functions: DataModelInitHelper (inline + src loading), GuardHelper
 *
 * W3C SCXML Features:
 * - ECMAScript whitespace normalization for non-XML content (B.2)
 * - External data loading via src attribute (5.2.2)
 * - String literal comparison in guard conditions
 */
struct Test558 : public SimpleAotTest<Test558, 558> {
    static constexpr const char *DESCRIPTION = "ECMAScript whitespace normalization (W3C B.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test558::test558;
};

// Auto-register
inline static AotTestRegistrar<Test558> registrar_Test558;

}  // namespace RSM::W3C::AotTests
