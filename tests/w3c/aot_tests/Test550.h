#pragma once
#include "SimpleAotTest.h"
#include "test550_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 5.2.2: Early binding with expr attribute for data variable
 *
 * Tests that the expr attribute can be used to assign a value to a variable with
 * early binding. Variable Var1 is initialized to value 2 from expression "2" before
 * state s0 is entered, and a conditional transition (cond="Var1 == 2") can reference
 * this initialized variable.
 *
 * W3C SCXML 5.2.2 specifies early binding semantics:
 * - Data elements are initialized at document initialization time
 * - Variables are available for use in conditional guards and executable content
 * - Even if a data element is declared in a state not in the active configuration,
 *   early binding ensures it is initialized at document start
 *
 * Test structure:
 * - State s1 contains <data id="Var1" expr="2"/>
 * - State s0 is the initial state (s1 is NOT entered)
 * - Despite s1 not being entered, Var1 is initialized due to early binding
 * - Eventless transition from s0 with cond="Var1 == 2" validates initialization
 *
 * Expected behavior:
 * 1. Document initialization: Var1 initialized to 2 (early binding)
 * 2. Enter s0 (initial state)
 * 3. Evaluate eventless transitions:
 *    - First transition: cond="Var1 == 2" → evaluates to true
 *    - Transition to pass state (final)
 * 4. State machine reaches final state: PASS
 *
 * This test ensures that early binding variable initialization occurs regardless
 * of whether the containing state is in the active configuration.
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * ✅ Static Hybrid Strategy:
 * - State machine structure: Fully static (compile-time known states/transitions)
 * - JSEngine for ECMAScript datamodel: Expression evaluation for data initialization and guards
 * - Uses Helper functions:
 *   - DataModelInitHelper: Initialize Var1 with expr="2" evaluation
 *   - GuardHelper: Evaluate conditional guard "Var1 == 2"
 *   - SystemVariableHelper: Setup _sessionid, _name, _ioprocessors
 * - Zero Duplication: Helper functions shared with Interpreter engine
 *
 * W3C SCXML Features:
 * - W3C SCXML 5.2.2: Early binding data initialization with expr attribute
 * - W3C SCXML B.2: ECMAScript datamodel for variable storage
 * - W3C SCXML E.1: Conditional expressions in transition guards
 * - W3C SCXML 3.3: Eventless transitions (null event processing)
 *
 * Key Implementation Detail:
 * Early binding requires all data elements to be initialized during document
 * initialization (StaticExecutionEngine::initialize()), before any states are
 * entered. DataModelInitHelper evaluates expr attributes in JSEngine context.
 */
struct Test550 : public SimpleAotTest<Test550, 550> {
    static constexpr const char *DESCRIPTION =
        "Early binding with expr attribute for data variable (W3C 5.2.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test550::test550;
};

// Auto-register
inline static AotTestRegistrar<Test550> registrar_Test550;

}  // namespace RSM::W3C::AotTests
