// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa3f7478a78abf9bf22f51a549ae822f834be956298adbc33316f195f470808d
// generated-at: 1781099318
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test286.scxml:1
package com.sce.w3c

import com.sce.generated.test286.Test286Event
import com.sce.generated.test286.Test286State
import com.sce.generated.test286.Test286StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.4: If the location expression of an assign does not denote a valid location in the datamodel the processor MUST place the error error.execution in the internal event queue.
@DisplayName("Test 286 -- W3C SCXML 5.4")
class Test286 : W3CTestBase<Test286State, Test286Event>() {
    override fun createStateMachine() = Test286StateMachine(createEngine())
    override val expectedPassState: Test286State = Test286State.Pass
}
