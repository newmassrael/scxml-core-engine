// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test309.scxml:1
package com.sce.w3c

import com.sce.generated.test309.Test309Event
import com.sce.generated.test309.Test309State
import com.sce.generated.test309.Test309StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If a conditional expression cannot be evaluated as a boolean value ('true' or 'false') or if its evaluation causes an error, the SCXML processor MUST treat the expression as if it evaluated to 'false'.
@DisplayName("Test 309 -- W3C SCXML 5.9")
class Test309 : W3CTestBase<Test309State, Test309Event>() {
    override fun createStateMachine() = Test309StateMachine(createEngine())
    override val expectedPassState: Test309State = Test309State.Pass
}
