// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test387.scxml:1
package com.sce.w3c

import com.sce.generated.test387.Test387Event
import com.sce.generated.test387.Test387State
import com.sce.generated.test387.Test387StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.10: Before the parent state has been visited for the first time, if a transition is executed that takes the history state as its target, the SCXML processor MUST behave as if the transition had taken the default stored state configuration as its target.
@DisplayName("Test 387 -- W3C SCXML 3.10")
class Test387 : W3CTestBase<Test387State, Test387Event>() {
    override fun createStateMachine() = Test387StateMachine()
    override val expectedPassState: Test387State = Test387State.Pass
}
