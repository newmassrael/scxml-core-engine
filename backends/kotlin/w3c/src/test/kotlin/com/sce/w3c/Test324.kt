// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test324.scxml:1
package com.sce.w3c

import com.sce.generated.test324.Test324Event
import com.sce.generated.test324.Test324State
import com.sce.generated.test324.Test324StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST keep the _name variable bound to the value of the 'name' attribute of the scxml element until the session terminates.
@DisplayName("Test 324 -- W3C SCXML 5.10")
class Test324 : W3CTestBase<Test324State, Test324Event>() {
    override fun createStateMachine() = Test324StateMachine(createEngine())
    override val expectedPassState: Test324State = Test324State.Pass
}
