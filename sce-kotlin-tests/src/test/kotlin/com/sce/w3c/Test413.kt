// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 168f4a554705bdfb42cc51a9fbd01e4e5fc028c49c4d6f47071af9577599e075
// generated-at: 1779449862
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
