// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d4e22b42c8fb86f84b0969d1316ae4d81aa27fbb4a4a2d70f49968ff55378fdc
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test216.scxml:1
package com.sce.w3c

import com.sce.generated.test216.Test216Event
import com.sce.generated.test216.Test216State
import com.sce.generated.test216.Test216StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the srcexpr attribute is present, the SCXML Processor MUST evaluate it when the parent invoke element is evaluated and treat the result as if it had been entered as the value of 'src'.
@DisplayName("Test 216 -- W3C SCXML 6.4")
class Test216 : W3CTestBase<Test216State, Test216Event>() {
    override fun createStateMachine() = Test216StateMachine(createEngine())
    override val expectedPassState: Test216State = Test216State.Pass
    override val timeoutMs: Long = 5000L
}
