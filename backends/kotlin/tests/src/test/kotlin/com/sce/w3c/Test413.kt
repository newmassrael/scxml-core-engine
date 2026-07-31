// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test413.scxml:1
package com.sce.w3c

import com.sce.generated.test413.Test413Event
import com.sce.generated.test413.Test413State
import com.sce.generated.test413.Test413StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: At startup, the SCXML Processor MUST place the state machine in the configuration specified by the 'initial' attribute of the scxml element.
@DisplayName("Test 413 -- W3C SCXML 3.13")
class Test413 : W3CTestBase<Test413State, Test413Event>() {
    override fun createStateMachine() = Test413StateMachine()
    override val expectedPassState: Test413State = Test413State.Pass
}
