// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 817d9c061804919d9748138703a11f334a156a4e2a1e5a3c66f1c4e7ca554aa2
// generated-at: 0
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
