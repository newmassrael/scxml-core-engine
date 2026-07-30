// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371281
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test525.scxml:1
package com.sce.w3c

import com.sce.generated.test525.Test525Event
import com.sce.generated.test525.Test525State
import com.sce.generated.test525.Test525StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: The SCXML processor MUST act as if it has made a shallow copy of the collection produced by the evaluation of 'array'. Specifically, modifications to the collection during the execution of foreach MUST NOT affect the iteration behavior.
@DisplayName("Test 525 -- W3C SCXML 4.6")
class Test525 : W3CTestBase<Test525State, Test525Event>() {
    override fun createStateMachine() = Test525StateMachine(createEngine())
    override val expectedPassState: Test525State = Test525State.Pass
}
