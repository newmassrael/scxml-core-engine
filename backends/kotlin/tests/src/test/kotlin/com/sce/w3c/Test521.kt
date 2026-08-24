// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 082e347ab97b9b491598f98d263b24d185e7e030b1c1600c8a0939850d86f8db
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
