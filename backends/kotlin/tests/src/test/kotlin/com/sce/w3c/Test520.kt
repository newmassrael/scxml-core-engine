// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: dbfa9cca1428438cf4178bb8fcf463f9b9d0c7c649f4bf0e0f3de90abcfd2a47
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test520.scxml:1
package com.sce.w3c

import com.sce.generated.test520.Test520Event
import com.sce.generated.test520.Test520State
import com.sce.generated.test520.Test520StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If a content child is present, the SCXML Processor MUST use its value as the body of the message.
@DisplayName("Test 520 -- W3C SCXML C.2")
class Test520 : W3CHttpTestBase<Test520State, Test520Event>() {
    override fun createStateMachine() = Test520StateMachine(createEngine())
    override val expectedPassState: Test520State = Test520State.Pass
}
