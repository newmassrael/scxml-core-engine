// SCE-GENERATED — DO NOT EDIT
// source-hash: 1036b5ee37fb5c89aaa1e8156f295f52aeb22d8997a046fafa16b7778f1ace69
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test233.scxml:1
package com.sce.w3c

import com.sce.generated.test233.Test233Event
import com.sce.generated.test233.Test233State
import com.sce.generated.test233.Test233StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If there is a finalize handler in the instance of invoke that created the service that generated the event, the SCXML Processor MUST execute the code in that finalize handler right before it removes the event from the event queue for processing.
@DisplayName("Test 233 -- W3C SCXML 6.4")
class Test233 : W3CTestBase<Test233State, Test233Event>() {
    override fun createStateMachine() = Test233StateMachine(createEngine())
    override val expectedPassState: Test233State = Test233State.Pass
}
