// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c96808b03e7b119d29792dbf258f9125c91be8c72d4823c8f9b56e0e05a3240b
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test452.scxml:1
package com.sce.w3c

import com.sce.generated.test452.Test452Event
import com.sce.generated.test452.Test452State
import com.sce.generated.test452.Test452StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, the SCXML Processor must accept any ECMAScript left-hand-side expression as a location expression.
@DisplayName("Test 452 -- W3C SCXML B.2")
class Test452 : W3CTestBase<Test452State, Test452Event>() {
    override fun createStateMachine() = Test452StateMachine(createEngine())
    override val expectedPassState: Test452State = Test452State.Pass
}
