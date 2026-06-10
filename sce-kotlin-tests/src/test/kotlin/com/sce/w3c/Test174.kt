// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa3f7478a78abf9bf22f51a549ae822f834be956298adbc33316f195f470808d
// generated-at: 1781099318
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test174.scxml:1
package com.sce.w3c

import com.sce.generated.test174.Test174Event
import com.sce.generated.test174.Test174State
import com.sce.generated.test174.Test174StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If 'typexpr' is present, the SCXML Processor MUST evaluate it when the parent send element is evaluated and treat the result as if it had been entered as the value of 'type'.
@DisplayName("Test 174 -- W3C SCXML 6.2")
class Test174 : W3CTestBase<Test174State, Test174Event>() {
    override fun createStateMachine() = Test174StateMachine(createEngine())
    override val expectedPassState: Test174State = Test174State.Pass
}
