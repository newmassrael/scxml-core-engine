// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2eaf0bfe80897ae515b0d732a8bb3914baa7c870ee8dd206a0a3dbc4956501d1
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test321.scxml:1
package com.sce.w3c

import com.sce.generated.test321.Test321Event
import com.sce.generated.test321.Test321State
import com.sce.generated.test321.Test321StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST bind the variable _sessionid at load time to the system-generated id for the current SCXML session.
@DisplayName("Test 321 -- W3C SCXML 5.10")
class Test321 : W3CTestBase<Test321State, Test321Event>() {
    override fun createStateMachine() = Test321StateMachine(createEngine())
    override val expectedPassState: Test321State = Test321State.Pass
}
