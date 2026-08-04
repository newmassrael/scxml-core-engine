// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: bf9f012bd8272e352f46f4d8064cf0cf3b743ab6fffdf8c941cc03f3254cb15f
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test550.scxml:1
package com.sce.w3c

import com.sce.generated.test550.Test550Event
import com.sce.generated.test550.Test550State
import com.sce.generated.test550.Test550StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: If the 'expr' attribute is present, the Platform MUST evaluate the corresponding expression at the time specified by the 'binding' attribute of scxml and MUST assign the resulting value as the value of the data element
@DisplayName("Test 550 -- W3C SCXML 5.3")
class Test550 : W3CTestBase<Test550State, Test550Event>() {
    override fun createStateMachine() = Test550StateMachine(createEngine())
    override val expectedPassState: Test550State = Test550State.Pass
}
