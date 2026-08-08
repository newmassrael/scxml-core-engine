// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 87472be6c3c3f32bb0fc76df8bdb613c176bab0c4f79720c0934dff4542328b8
// generated-at: 0
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
