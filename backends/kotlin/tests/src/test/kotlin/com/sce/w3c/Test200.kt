// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: bf9f012bd8272e352f46f4d8064cf0cf3b743ab6fffdf8c941cc03f3254cb15f
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test200.scxml:1
package com.sce.w3c

import com.sce.generated.test200.Test200Event
import com.sce.generated.test200.Test200State
import com.sce.generated.test200.Test200StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: SCXML Processors MUST support the type http://www.w3.org/TR/scxml/#SCXMLEventProcessor
@DisplayName("Test 200 -- W3C SCXML 6.2")
class Test200 : W3CTestBase<Test200State, Test200Event>() {
    override fun createStateMachine() = Test200StateMachine()
    override val expectedPassState: Test200State = Test200State.Pass
}
