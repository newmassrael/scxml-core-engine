// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: c6c9654e14987bf9fee21998d111ca1385c48c09f2deb9cc862525d124525214
// generated-at: 1785480867
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test409.scxml:1
package com.sce.w3c

import com.sce.generated.test409.Test409Event
import com.sce.generated.test409.Test409State
import com.sce.generated.test409.Test409StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: Finally [after the onexits and canceling the invocations], the Processor MUST remove the state from the active state's list.
@DisplayName("Test 409 -- W3C SCXML 3.13")
class Test409 : W3CTestBase<Test409State, Test409Event>() {
    override fun createStateMachine() = Test409StateMachine()
    override val expectedPassState: Test409State = Test409State.Pass
    override val timeoutMs: Long = 5000L
}
