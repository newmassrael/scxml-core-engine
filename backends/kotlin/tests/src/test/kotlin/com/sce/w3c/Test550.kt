// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d4e22b42c8fb86f84b0969d1316ae4d81aa27fbb4a4a2d70f49968ff55378fdc
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
