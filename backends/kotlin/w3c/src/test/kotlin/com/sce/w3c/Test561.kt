// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test561.scxml:1
package com.sce.w3c

import com.sce.generated.test561.Test561Event
import com.sce.generated.test561.Test561State
import com.sce.generated.test561.Test561StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, if the content provided to populate _event.data can be interpreted as a valid XML document
@DisplayName("Test 561 -- W3C SCXML B.2")
class Test561 : W3CTestBase<Test561State, Test561Event>() {
    override fun createStateMachine() = Test561StateMachine(createEngine())
    override val expectedPassState: Test561State = Test561State.Pass
}
