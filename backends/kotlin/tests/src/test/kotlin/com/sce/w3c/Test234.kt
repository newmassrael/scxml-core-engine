// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0039966e0f3716b85eeb59960e8ad41f86b7aa3caf1343b6b830b8699ccc194e
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test234.scxml:1
package com.sce.w3c

import com.sce.generated.test234.Test234Event
import com.sce.generated.test234.Test234State
import com.sce.generated.test234.Test234StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: t MUST NOT execute the finalize handler in any other instance of invoke besides the one in the instance of invoke that created the service that generated the event.
@DisplayName("Test 234 -- W3C SCXML 6.4")
class Test234 : W3CTestBase<Test234State, Test234Event>() {
    override fun createStateMachine() = Test234StateMachine(createEngine())
    override val expectedPassState: Test234State = Test234State.Pass
}
