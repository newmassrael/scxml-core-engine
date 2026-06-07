// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 3acf03cd1e197da0d6a3e7ecc2541747678939372fbe1d99b37c7415a38be32a
// generated-at: 1780830703
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
