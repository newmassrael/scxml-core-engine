// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 039ad389d30ffb729c7c2441931b41f36924cbf4b6013115d42ef3094467532b
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test155.scxml:1
package com.sce.w3c

import com.sce.generated.test155.Test155Event
import com.sce.generated.test155.Test155State
import com.sce.generated.test155.Test155StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: when evaluating foreach, for each item, after making the assignment, the SCXML processor MUST evaluate its child executable content. It MUST then proceed to the next item in iteration order.
@DisplayName("Test 155 -- W3C SCXML 4.6")
class Test155 : W3CTestBase<Test155State, Test155Event>() {
    override fun createStateMachine() = Test155StateMachine(createEngine())
    override val expectedPassState: Test155State = Test155State.Pass
}
