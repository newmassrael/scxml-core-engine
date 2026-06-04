// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test377.scxml:1
package com.sce.w3c

import com.sce.generated.test377.Test377Event
import com.sce.generated.test377.Test377State
import com.sce.generated.test377.Test377StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.9: The SCXML processor MUST execute the onexit handlers of a state in document order when the state is exited.
@DisplayName("Test 377 -- W3C SCXML 3.9")
class Test377 : W3CTestBase<Test377State, Test377Event>() {
    override fun createStateMachine() = Test377StateMachine()
    override val expectedPassState: Test377State = Test377State.Pass
}
