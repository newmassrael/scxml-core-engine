// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test179.scxml:1
package com.sce.w3c

import com.sce.generated.test179.Test179Event
import com.sce.generated.test179.Test179State
import com.sce.generated.test179.Test179StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: The SCXML Processor MUST evaluate the content element when the parent send element is evaluated and pass the resulting data unmodified to the external service when the message is delivered.
@DisplayName("Test 179 -- W3C SCXML 6.2")
class Test179 : W3CTestBase<Test179State, Test179Event>() {
    override fun createStateMachine() = Test179StateMachine(createEngine())
    override val expectedPassState: Test179State = Test179State.Pass
}
