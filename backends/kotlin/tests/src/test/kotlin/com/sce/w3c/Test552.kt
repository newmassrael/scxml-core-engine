// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2337021aa5cf9b8209b5932f23ab0e04a6899271e435f3620bc1da41d7c4d7b7
// generated-at: 1784381545
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test552.scxml:1
package com.sce.w3c

import com.sce.generated.test552.Test552Event
import com.sce.generated.test552.Test552State
import com.sce.generated.test552.Test552StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: If the 'src' attribute is present, the Platform MUST fetch the specified object at the time specified by the 'binding' attribute of scxml and MUST assign it as the value of the data element.
@DisplayName("Test 552 -- W3C SCXML 5.3")
class Test552 : W3CTestBase<Test552State, Test552Event>() {
    override fun createStateMachine() = Test552StateMachine(createEngine())
    override val expectedPassState: Test552State = Test552State.Pass
}
