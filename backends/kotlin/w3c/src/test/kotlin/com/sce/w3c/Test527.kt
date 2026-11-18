// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
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
