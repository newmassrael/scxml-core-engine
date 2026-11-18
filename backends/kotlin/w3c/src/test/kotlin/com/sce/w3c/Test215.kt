// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test215.scxml:1
package com.sce.w3c

import com.sce.generated.test215.Test215Event
import com.sce.generated.test215.Test215State
import com.sce.generated.test215.Test215StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the typeexpr attribute is present, the SCXML Processor MUST evaluate it when the parent invoke element is evaluated and treat the result as if it had been entered as the value of 'type'.
@DisplayName("Test 215 -- W3C SCXML 6.4")
class Test215 : W3CTestBase<Test215State, Test215Event>() {
    override fun createStateMachine() = Test215StateMachine(createEngine())
    override val expectedPassState: Test215State = Test215State.Pass
}
