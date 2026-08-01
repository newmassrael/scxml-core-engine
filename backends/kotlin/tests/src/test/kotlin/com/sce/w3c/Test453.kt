// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0039966e0f3716b85eeb59960e8ad41f86b7aa3caf1343b6b830b8699ccc194e
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test453.scxml:1
package com.sce.w3c

import com.sce.generated.test453.Test453Event
import com.sce.generated.test453.Test453State
import com.sce.generated.test453.Test453StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel,  the SCXML Processor must accept any ECMAScript expression as a value expression.
@DisplayName("Test 453 -- W3C SCXML B.2")
class Test453 : W3CTestBase<Test453State, Test453Event>() {
    override fun createStateMachine() = Test453StateMachine(createEngine())
    override val expectedPassState: Test453State = Test453State.Pass
}
