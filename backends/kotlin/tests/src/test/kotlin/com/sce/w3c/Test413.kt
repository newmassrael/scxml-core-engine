// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 057f3064c2c620977191e86f67c1d505edec850a0d81b50b27d4b101952af703
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
