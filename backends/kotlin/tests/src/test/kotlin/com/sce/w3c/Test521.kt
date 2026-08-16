// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 128f5bda1db8a8695e204b38e87b8d2d3815bdde9691186823a5ecdc7374af1d
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test521.scxml:1
package com.sce.w3c

import com.sce.generated.test521.Test521Event
import com.sce.generated.test521.Test521State
import com.sce.generated.test521.Test521StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: f the Processor cannot dispatch the event, it MUST place the error error.communication on the internal event queue of the session that attempted to send the event.
@DisplayName("Test 521 -- W3C SCXML 6.2")
class Test521 : W3CTestBase<Test521State, Test521Event>() {
    override fun createStateMachine() = Test521StateMachine(createEngine())
    override val expectedPassState: Test521State = Test521State.Pass
}
