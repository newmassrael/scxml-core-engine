// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9faef2370910e1d1b12ff0b00a3d63d3578977b6f3f2045b8b014f47fa072349
// generated-at: 1778932425
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
