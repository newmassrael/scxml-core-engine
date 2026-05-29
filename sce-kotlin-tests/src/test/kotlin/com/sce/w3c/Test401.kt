// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5af0768adc0cd444b401fc40536c0de87cadf9b1f8be7299536f4fc9ed22e337
// generated-at: 1780020098
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test401.scxml:1
package com.sce.w3c

import com.sce.generated.test401.Test401Event
import com.sce.generated.test401.Test401State
import com.sce.generated.test401.Test401StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.12: The processor MUST place these [error] events in the internal event queue.
@DisplayName("Test 401 -- W3C SCXML 3.12")
class Test401 : W3CTestBase<Test401State, Test401Event>() {
    override fun createStateMachine() = Test401StateMachine(createEngine())
    override val expectedPassState: Test401State = Test401State.Pass
}
