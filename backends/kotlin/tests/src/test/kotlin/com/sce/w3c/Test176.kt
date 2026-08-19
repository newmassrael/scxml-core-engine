// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 10c5bb56d60f6d5bc4121611a1230324eaf61d1a5524b71d52c6010f279d5ffd
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test176.scxml:1
package com.sce.w3c

import com.sce.generated.test176.Test176Event
import com.sce.generated.test176.Test176State
import com.sce.generated.test176.Test176StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: The SCXML Processor MUST evaluate param when the parent send element is evaluated and pass the resulting data unmodified to the external service when the message is delivered
@DisplayName("Test 176 -- W3C SCXML 6.2")
class Test176 : W3CTestBase<Test176State, Test176Event>() {
    override fun createStateMachine() = Test176StateMachine(createEngine())
    override val expectedPassState: Test176State = Test176State.Pass
}
