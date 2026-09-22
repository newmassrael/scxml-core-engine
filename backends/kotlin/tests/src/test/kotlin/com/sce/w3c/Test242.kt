// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 583c0d21905ba9b04748a3d6c74f6cc9796a6884a4c31fd1817c64ee900ca1d2
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test242.scxml:1
package com.sce.w3c

import com.sce.generated.test242.Test242Event
import com.sce.generated.test242.Test242State
import com.sce.generated.test242.Test242StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Invoked services MUST also treat values specified by 'src' and content identically.
@DisplayName("Test 242 -- W3C SCXML 6.4")
class Test242 : W3CTestBase<Test242State, Test242Event>() {
    override fun createStateMachine() = Test242StateMachine()
    override val expectedPassState: Test242State = Test242State.Pass
    override val timeoutMs: Long = 5000L
}
