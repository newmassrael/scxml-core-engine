// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c56c70704f5f2bb35214b9afedb17072586ef770e8f707a00c3cc931286b4b25
// generated-at: 0
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
