// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 63129ea5a60cce4407210a3c2e3ff224327767ebf6618c3f4ed41b0a49b7454d
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test528.scxml:1
package com.sce.w3c

import com.sce.generated.test528.Test528Event
import com.sce.generated.test528.Test528State
import com.sce.generated.test528.Test528StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.6: f the evaluation of 'expr' produces an error, the Processor MUST place error.execution in the internal event queue and use the empty string as the output of the content element.
@DisplayName("Test 528 -- W3C SCXML 5.6")
class Test528 : W3CTestBase<Test528State, Test528Event>() {
    override fun createStateMachine() = Test528StateMachine(createEngine())
    override val expectedPassState: Test528State = Test528State.Pass
}
