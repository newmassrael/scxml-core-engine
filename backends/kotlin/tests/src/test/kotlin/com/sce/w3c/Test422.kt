// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 615c09cf1e666fafc78d1f8f6d6f319491336c3f372af9d38785e88a213f5256
// generated-at: 1785425248
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test422.scxml:1
package com.sce.w3c

import com.sce.generated.test422.Test422Event
import com.sce.generated.test422.Test422State
import com.sce.generated.test422.Test422StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: After completing a macrostep, the SCXML Processor MUST execute in document order the invoke handlers in all states that have been entered (and not exited) since the completion of the last macrostep.
@DisplayName("Test 422 -- W3C SCXML 3.13")
class Test422 : W3CTestBase<Test422State, Test422Event>() {
    override fun createStateMachine() = Test422StateMachine(createEngine())
    override val expectedPassState: Test422State = Test422State.Pass
    override val timeoutMs: Long = 5000L
}
