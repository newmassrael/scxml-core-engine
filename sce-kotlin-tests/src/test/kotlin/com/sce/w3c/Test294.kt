// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d2f0bcf4d5c727ad2446a904193402929b9b2d65dfec5e5c07ad3bc881483b09
// generated-at: 1780358475
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test294.scxml:1
package com.sce.w3c

import com.sce.generated.test294.Test294Event
import com.sce.generated.test294.Test294State
import com.sce.generated.test294.Test294StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.5: In cases where the SCXML Processor generates a 'done' event upon entry into the final state, it MUST evaluate the donedata elements param or content children and place the resulting data in the _event.data field. The exact format of that data will be determined by the datamodel
@DisplayName("Test 294 -- W3C SCXML 5.5")
class Test294 : W3CTestBase<Test294State, Test294Event>() {
    override fun createStateMachine() = Test294StateMachine(createEngine())
    override val expectedPassState: Test294State = Test294State.Pass
}
