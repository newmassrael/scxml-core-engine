// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371281
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
