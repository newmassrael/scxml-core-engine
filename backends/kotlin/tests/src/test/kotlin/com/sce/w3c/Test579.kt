// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: c6c9654e14987bf9fee21998d111ca1385c48c09f2deb9cc862525d124525214
// generated-at: 1785480867
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test579.scxml:1
package com.sce.w3c

import com.sce.generated.test579.Test579Event
import com.sce.generated.test579.Test579State
import com.sce.generated.test579.Test579StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.10: Before the parent state has been visited for the first time, if a transition is executed that takes the history state as its target,
@DisplayName("Test 579 -- W3C SCXML 3.10")
class Test579 : W3CTestBase<Test579State, Test579Event>() {
    override fun createStateMachine() = Test579StateMachine(createEngine())
    override val expectedPassState: Test579State = Test579State.Pass
    override val timeoutMs: Long = 5000L
}
