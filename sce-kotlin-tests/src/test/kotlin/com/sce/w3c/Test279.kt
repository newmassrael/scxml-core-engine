// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bc7b5b1dd90f65e6c3a4df2e3c4223cf8922d7e6b2d5d124b66683d16074cb6e
// generated-at: 1780362263
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
