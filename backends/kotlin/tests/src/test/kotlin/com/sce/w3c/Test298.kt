// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 63129ea5a60cce4407210a3c2e3ff224327767ebf6618c3f4ed41b0a49b7454d
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test298.scxml:1
package com.sce.w3c

import com.sce.generated.test298.Test298Event
import com.sce.generated.test298.Test298State
import com.sce.generated.test298.Test298StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.7: If the 'location' attribute on a param element does not refer to a valid location in the data model, the processor MUST place the error error.execution on the internal event queue.
@DisplayName("Test 298 -- W3C SCXML 5.7")
class Test298 : W3CTestBase<Test298State, Test298Event>() {
    override fun createStateMachine() = Test298StateMachine(createEngine())
    override val expectedPassState: Test298State = Test298State.Pass
}
