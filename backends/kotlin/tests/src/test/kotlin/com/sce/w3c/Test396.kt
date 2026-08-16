// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 128f5bda1db8a8695e204b38e87b8d2d3815bdde9691186823a5ecdc7374af1d
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test396.scxml:1
package com.sce.w3c

import com.sce.generated.test396.Test396Event
import com.sce.generated.test396.Test396State
import com.sce.generated.test396.Test396StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.12: The SCXML processor MUST use this same name value [the one reflected in the event variable] to match against the 'event' attribute of transitions.
@DisplayName("Test 396 -- W3C SCXML 3.12")
class Test396 : W3CTestBase<Test396State, Test396Event>() {
    override fun createStateMachine() = Test396StateMachine()
    override val expectedPassState: Test396State = Test396State.Pass
}
