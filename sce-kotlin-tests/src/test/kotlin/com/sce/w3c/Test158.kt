// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa3f7478a78abf9bf22f51a549ae822f834be956298adbc33316f195f470808d
// generated-at: 1781102364
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test158.scxml:1
package com.sce.w3c

import com.sce.generated.test158.Test158Event
import com.sce.generated.test158.Test158State
import com.sce.generated.test158.Test158StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.9: The SCXML processor MUST execute the elements of a block of executable contentin document order.
@DisplayName("Test 158 -- W3C SCXML 4.9")
class Test158 : W3CTestBase<Test158State, Test158Event>() {
    override fun createStateMachine() = Test158StateMachine(createEngine())
    override val expectedPassState: Test158State = Test158State.Pass
}
