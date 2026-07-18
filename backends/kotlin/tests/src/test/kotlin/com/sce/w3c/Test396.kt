// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2337021aa5cf9b8209b5932f23ab0e04a6899271e435f3620bc1da41d7c4d7b7
// generated-at: 1784381545
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
