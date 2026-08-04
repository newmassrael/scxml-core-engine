// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: bf9f012bd8272e352f46f4d8064cf0cf3b743ab6fffdf8c941cc03f3254cb15f
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test309.scxml:1
package com.sce.w3c

import com.sce.generated.test309.Test309Event
import com.sce.generated.test309.Test309State
import com.sce.generated.test309.Test309StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If a conditional expression cannot be evaluated as a boolean value ('true' or 'false') or if its evaluation causes an error, the SCXML processor MUST treat the expression as if it evaluated to 'false'.
@DisplayName("Test 309 -- W3C SCXML 5.9")
class Test309 : W3CTestBase<Test309State, Test309Event>() {
    override fun createStateMachine() = Test309StateMachine(createEngine())
    override val expectedPassState: Test309State = Test309State.Pass
}
