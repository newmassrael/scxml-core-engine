// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test242.scxml:1
package com.sce.w3c

import com.sce.generated.test242.Test242Event
import com.sce.generated.test242.Test242State
import com.sce.generated.test242.Test242StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Invoked services MUST also treat values specified by 'src' and content identically.
@DisplayName("Test 242 -- W3C SCXML 6.4")
class Test242 : W3CTestBase<Test242State, Test242Event>() {
    override fun createStateMachine() = Test242StateMachine()
    override val expectedPassState: Test242State = Test242State.Pass
    override val timeoutMs: Long = 5000L
}
