// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f8628fc45ae1ba8d3b0272fbb37ab2b3fa73e6bcc8f28ed51f64ec3e41941c33
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
