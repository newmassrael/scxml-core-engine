// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 670395eefe7272d78e62bf7a7fd9181e96e4a744175a58a4c4de1240c73f57bc
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test336.scxml:1
package com.sce.w3c

import com.sce.generated.test336.Test336Event
import com.sce.generated.test336.Test336State
import com.sce.generated.test336.Test336StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: For external events, the SCXML Processor SHOULD set the origintype field to a value which, in combination with the 'origin' field, will allow the receiver of the event to send a response back to the originating entity.
@DisplayName("Test 336 -- W3C SCXML 5.10")
class Test336 : W3CTestBase<Test336State, Test336Event>() {
    override fun createStateMachine() = Test336StateMachine(createEngine())
    override val expectedPassState: Test336State = Test336State.Pass
}
