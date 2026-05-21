// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9c6a4f8dfaed131dd8c4550407375e80f92b4e4373728b55d22f59422722a6ba
// generated-at: 1779372462
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test349.scxml:1
package com.sce.w3c

import com.sce.generated.test349.Test349Event
import com.sce.generated.test349.Test349State
import com.sce.generated.test349.Test349StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: source'. The sending SCXML Processor MUST populate this attribute with a URI that the receiving processor can use to reply to the sending processor. The receiving SCXML Processor MUST use this URI as the value of the 'origin' field in the event that it generates.
@DisplayName("Test 349 -- W3C SCXML C.1")
class Test349 : W3CTestBase<Test349State, Test349Event>() {
    override fun createStateMachine() = Test349StateMachine(createEngine())
    override val expectedPassState: Test349State = Test349State.Pass
}
