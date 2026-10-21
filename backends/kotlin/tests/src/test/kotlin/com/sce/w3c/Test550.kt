// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
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
