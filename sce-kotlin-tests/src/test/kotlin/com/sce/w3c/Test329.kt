// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa3f7478a78abf9bf22f51a549ae822f834be956298adbc33316f195f470808d
// generated-at: 1781099318
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test329.scxml:1
package com.sce.w3c

import com.sce.generated.test329.Test329Event
import com.sce.generated.test329.Test329State
import com.sce.generated.test329.Test329StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST cause any attempt to change the value of a system variable to fail.
@DisplayName("Test 329 -- W3C SCXML 5.10")
class Test329 : W3CTestBase<Test329State, Test329Event>() {
    override fun createStateMachine() = Test329StateMachine(createEngine())
    override val expectedPassState: Test329State = Test329State.Pass
}
