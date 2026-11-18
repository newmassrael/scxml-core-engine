// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test303.scxml:1
package com.sce.w3c

import com.sce.generated.test303.Test303Event
import com.sce.generated.test303.Test303State
import com.sce.generated.test303.Test303StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.8: The SCXML Processor MUST evaluate all script elements not children of scxml as part of normal executable content evaluation. N.B. This test is valid only for datamodels that support scripting.
@DisplayName("Test 303 -- W3C SCXML 5.8")
class Test303 : W3CTestBase<Test303State, Test303Event>() {
    override fun createStateMachine() = Test303StateMachine(createEngine())
    override val expectedPassState: Test303State = Test303State.Pass
}
