// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test376.scxml:1
package com.sce.w3c

import com.sce.generated.test376.Test376Event
import com.sce.generated.test376.Test376State
import com.sce.generated.test376.Test376StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.8: The SCXML processor MUST treat each [onentry] handler as a separate block of executable content.
@DisplayName("Test 376 -- W3C SCXML 3.8")
class Test376 : W3CTestBase<Test376State, Test376Event>() {
    override fun createStateMachine() = Test376StateMachine(createEngine())
    override val expectedPassState: Test376State = Test376State.Pass
}
