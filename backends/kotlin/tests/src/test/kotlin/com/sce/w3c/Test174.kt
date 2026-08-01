// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0039966e0f3716b85eeb59960e8ad41f86b7aa3caf1343b6b830b8699ccc194e
// generated-at: 0
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
