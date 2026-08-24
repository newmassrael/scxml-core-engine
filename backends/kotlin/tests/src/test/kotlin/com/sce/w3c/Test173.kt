// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 6d29ccd65cc69c7036210e21d4c9d2a46b7717262dc7e045f86a45620f80383f
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test173.scxml:1
package com.sce.w3c

import com.sce.generated.test173.Test173Event
import com.sce.generated.test173.Test173State
import com.sce.generated.test173.Test173StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If 'targetexpr' is present, the SCXML Processor MUST evaluate it when the parent send element is evaluated and treat the result as if it had been entered as the value of 'target'.
@DisplayName("Test 173 -- W3C SCXML 6.2")
class Test173 : W3CTestBase<Test173State, Test173Event>() {
    override fun createStateMachine() = Test173StateMachine(createEngine())
    override val expectedPassState: Test173State = Test173State.Pass
}
