// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa3f7478a78abf9bf22f51a549ae822f834be956298adbc33316f195f470808d
// generated-at: 1781099318
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test551.scxml:1
package com.sce.w3c

import com.sce.generated.test551.Test551Event
import com.sce.generated.test551.Test551State
import com.sce.generated.test551.Test551StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: f child content is specified, the Platform MUST assign it as the value of the data element at the time specified by the 'binding' attribute of scxml.
@DisplayName("Test 551 -- W3C SCXML 5.3")
class Test551 : W3CTestBase<Test551State, Test551Event>() {
    override fun createStateMachine() = Test551StateMachine(createEngine())
    override val expectedPassState: Test551State = Test551State.Pass
}
