// GENERATED -- DO NOT EDIT (sce-codegen)
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
