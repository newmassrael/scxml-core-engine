// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
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
