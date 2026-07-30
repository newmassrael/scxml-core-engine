// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371281
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
