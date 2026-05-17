// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c1736039ea6628ae1068e428522a9d89bbe2ccef2705503db256c49ec169955e
// generated-at: 1778992486
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test279.scxml:1
package com.sce.w3c

import com.sce.generated.test279.Test279Event
import com.sce.generated.test279.Test279State
import com.sce.generated.test279.Test279StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: When 'binding' attribute on the scxml element is assigned the value "early" (the default), the SCXML Processor MUST create all data elements and assign their initial values at document initialization time.
@DisplayName("Test 279 -- W3C SCXML 5.3")
class Test279 : W3CTestBase<Test279State, Test279Event>() {
    override fun createStateMachine() = Test279StateMachine(createEngine())
    override val expectedPassState: Test279State = Test279State.Pass
}
