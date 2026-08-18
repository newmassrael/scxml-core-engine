// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 6316b28d6d1ad6f020128cbcac8380bcbacc703c59c413c6fbd546c800047e63
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test527.scxml:1
package com.sce.w3c

import com.sce.generated.test527.Test527Event
import com.sce.generated.test527.Test527State
import com.sce.generated.test527.Test527StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.6: When the SCXML Processor evaluates the content element, if the 'expr' value expression is present, the Processor MUST evaluate it and use the result as the output of the content element.
@DisplayName("Test 527 -- W3C SCXML 5.6")
class Test527 : W3CTestBase<Test527State, Test527Event>() {
    override fun createStateMachine() = Test527StateMachine(createEngine())
    override val expectedPassState: Test527State = Test527State.Pass
}
