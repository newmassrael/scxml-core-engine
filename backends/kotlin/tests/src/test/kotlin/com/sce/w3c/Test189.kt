// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: dbfa9cca1428438cf4178bb8fcf463f9b9d0c7c649f4bf0e0f3de90abcfd2a47
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test189.scxml:1
package com.sce.w3c

import com.sce.generated.test189.Test189Event
import com.sce.generated.test189.Test189State
import com.sce.generated.test189.Test189StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: When using the scxml event i/o processor] If the target is the special term '#_internal', the Processor MUST add the event to the internal event queue of the sending session
@DisplayName("Test 189 -- W3C SCXML C.1")
class Test189 : W3CTestBase<Test189State, Test189Event>() {
    override fun createStateMachine() = Test189StateMachine()
    override val expectedPassState: Test189State = Test189State.Pass
}
