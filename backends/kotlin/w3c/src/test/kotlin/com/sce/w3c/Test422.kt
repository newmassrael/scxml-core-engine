// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
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
